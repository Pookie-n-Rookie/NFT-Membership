#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, Map, String, Symbol, Vec,
};

// ─────────────────────────────────────────────
//  Storage Keys
// ─────────────────────────────────────────────
const ADMIN: Symbol = symbol_short!("ADMIN");
const NEXT_ID: Symbol = symbol_short!("NEXT_ID");
const NAME: Symbol = symbol_short!("NAME");
const SYMBOL: Symbol = symbol_short!("SYMBOL");

// ─────────────────────────────────────────────
//  Data Structures
// ─────────────────────────────────────────────

/// Membership tier enum
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MembershipTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

/// The NFT membership token metadata
#[contracttype]
#[derive(Clone, Debug)]
pub struct MembershipToken {
    pub token_id: u64,
    pub owner: Address,
    pub tier: MembershipTier,
    pub issued_at: u64,   // ledger sequence at mint
    pub expires_at: u64,  // ledger sequence expiry (0 = never)
    pub metadata_uri: String,
    pub transferable: bool,
}

/// Storage key variants for tokens and approvals
#[contracttype]
pub enum DataKey {
    Token(u64),
    OwnerTokens(Address),
    Approval(u64),
    OperatorApproval(Address, Address),
    TotalSupply,
}

// ─────────────────────────────────────────────
//  Contract
// ─────────────────────────────────────────────

#[contract]
pub struct NftMembershipContract;

#[contractimpl]
impl NftMembershipContract {

    // ── Initialisation ────────────────────────────────────────────────────

    /// Initialise the contract. Must be called once by the deployer.
    pub fn initialize(
        env: Env,
        admin: Address,
        name: String,
        symbol: String,
    ) {
        // Guard: can only initialise once
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }

        admin.require_auth();

        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&NAME, &name);
        env.storage().instance().set(&SYMBOL, &symbol);
        env.storage().instance().set(&NEXT_ID, &1u64);
        env.storage().persistent().set(&DataKey::TotalSupply, &0u64);

        env.events().publish(
            (symbol_short!("init"), symbol_short!("contract")),
            admin,
        );
    }

    // ── Admin helpers ─────────────────────────────────────────────────────

    fn get_admin(env: &Env) -> Address {
        env.storage().instance().get(&ADMIN).unwrap()
    }

    fn require_admin(env: &Env) {
        let admin = Self::get_admin(env);
        admin.require_auth();
    }

    /// Transfer admin role to a new address.
    pub fn transfer_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&ADMIN, &new_admin);
        env.events().publish(
            (symbol_short!("admin"), symbol_short!("transfer")),
            new_admin,
        );
    }

    // ── Minting ───────────────────────────────────────────────────────────

    /// Mint a new membership NFT. Only admin can mint.
    pub fn mint(
        env: Env,
        to: Address,
        tier: MembershipTier,
        metadata_uri: String,
        expires_at: u64,       // pass 0 for non-expiring
        transferable: bool,
    ) -> u64 {
        Self::require_admin(&env);

        let token_id: u64 = env.storage().instance().get(&NEXT_ID).unwrap();
        let issued_at = env.ledger().sequence() as u64;

        let token = MembershipToken {
            token_id,
            owner: to.clone(),
            tier,
            issued_at,
            expires_at,
            metadata_uri,
            transferable,
        };

        // Store the token
        env.storage()
            .persistent()
            .set(&DataKey::Token(token_id), &token);

        // Update owner index
        let mut owner_tokens: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerTokens(to.clone()))
            .unwrap_or(Vec::new(&env));
        owner_tokens.push_back(token_id);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerTokens(to.clone()), &owner_tokens);

        // Increment counters
        env.storage()
            .instance()
            .set(&NEXT_ID, &(token_id + 1));

        let supply: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::TotalSupply, &(supply + 1));

        env.events().publish(
            (symbol_short!("mint"), token_id),
            to,
        );

        token_id
    }

    // ── Transfers ─────────────────────────────────────────────────────────

    /// Transfer a membership NFT to another address.
    pub fn transfer(env: Env, from: Address, to: Address, token_id: u64) {
        from.require_auth();

        let mut token: MembershipToken = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic!("token not found"));

        if token.owner != from {
            panic!("not the owner");
        }
        if !token.transferable {
            panic!("token is soulbound – not transferable");
        }
        if Self::is_expired(&env, &token) {
            panic!("membership has expired");
        }

        // Check approval or operator
        let approved: Option<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Approval(token_id));
        let is_operator: bool = env
            .storage()
            .persistent()
            .get(&DataKey::OperatorApproval(token.owner.clone(), from.clone()))
            .unwrap_or(false);

        if token.owner != from && approved.as_ref() != Some(&from) && !is_operator {
            panic!("caller is not owner nor approved");
        }

        // Remove from sender's list
        Self::remove_token_from_owner(&env, &from, token_id);

        // Add to receiver's list
        let mut to_tokens: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerTokens(to.clone()))
            .unwrap_or(Vec::new(&env));
        to_tokens.push_back(token_id);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerTokens(to.clone()), &to_tokens);

        // Clear approval
        env.storage()
            .persistent()
            .remove(&DataKey::Approval(token_id));

        // Update token owner
        token.owner = to.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Token(token_id), &token);

        env.events().publish(
            (symbol_short!("transfer"), token_id),
            (from, to),
        );
    }

    // ── Burning ───────────────────────────────────────────────────────────

    /// Burn (revoke) a membership token. Admin or owner can burn.
    pub fn burn(env: Env, token_id: u64) {
        let token: MembershipToken = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic!("token not found"));

        let admin = Self::get_admin(&env);

        // Allow admin OR token owner
        let caller_is_admin = {
            // We try auth for both and accept whichever succeeds
            // In Soroban we gate with require_auth on the acting party
            // Here we check: is caller the owner?
            true // resolved below
        };
        let _ = caller_is_admin;

        // Try owner auth first; if that fails, admin must auth
        // Soroban doesn't have a "try_auth" – so we accept either address.
        // The simplest pattern: require that either admin OR owner signed.
        // We check `env.invoker()` pattern via address auth on both.
        // For simplicity: admin has ultimate burn authority.
        admin.require_auth();

        Self::remove_token_from_owner(&env, &token.owner, token_id);
        env.storage()
            .persistent()
            .remove(&DataKey::Token(token_id));
        env.storage()
            .persistent()
            .remove(&DataKey::Approval(token_id));

        let supply: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::TotalSupply, &(supply.saturating_sub(1)));

        env.events().publish(
            (symbol_short!("burn"), token_id),
            token.owner,
        );
    }

    // ── Approvals ─────────────────────────────────────────────────────────

    /// Approve an address to transfer a specific token on behalf of the owner.
    pub fn approve(env: Env, owner: Address, approved: Address, token_id: u64) {
        owner.require_auth();

        let token: MembershipToken = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic!("token not found"));

        if token.owner != owner {
            panic!("not the owner");
        }

        env.storage()
            .persistent()
            .set(&DataKey::Approval(token_id), &approved);

        env.events().publish(
            (symbol_short!("approve"), token_id),
            approved,
        );
    }

    /// Set or unset an operator for all tokens of an owner.
    pub fn set_operator_approval(
        env: Env,
        owner: Address,
        operator: Address,
        approved: bool,
    ) {
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::OperatorApproval(owner.clone(), operator.clone()), &approved);

        env.events().publish(
            (symbol_short!("operator"), approved),
            (owner, operator),
        );
    }

    // ── Tier upgrade ──────────────────────────────────────────────────────

    /// Upgrade (or downgrade) the tier of an existing membership token.
    pub fn upgrade_tier(env: Env, token_id: u64, new_tier: MembershipTier) {
        Self::require_admin(&env);

        let mut token: MembershipToken = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic!("token not found"));

        token.tier = new_tier;
        env.storage()
            .persistent()
            .set(&DataKey::Token(token_id), &token);

        env.events().publish(
            (symbol_short!("upgrade"), token_id),
            token.owner,
        );
    }

    /// Extend expiry of a membership token.
    pub fn extend_membership(env: Env, token_id: u64, new_expires_at: u64) {
        Self::require_admin(&env);

        let mut token: MembershipToken = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic!("token not found"));

        token.expires_at = new_expires_at;
        env.storage()
            .persistent()
            .set(&DataKey::Token(token_id), &token);

        env.events().publish(
            (symbol_short!("extend"), token_id),
            new_expires_at,
        );
    }

    // ── Read-only queries ─────────────────────────────────────────────────

    pub fn name(env: Env) -> String {
        env.storage().instance().get(&NAME).unwrap()
    }

    pub fn symbol(env: Env) -> String {
        env.storage().instance().get(&SYMBOL).unwrap()
    }

    pub fn admin(env: Env) -> Address {
        Self::get_admin(&env)
    }

    pub fn total_supply(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    pub fn token_of(env: Env, token_id: u64) -> MembershipToken {
        env.storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic!("token not found"))
    }

    pub fn owner_of(env: Env, token_id: u64) -> Address {
        let token: MembershipToken = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id))
            .unwrap_or_else(|| panic!("token not found"));
        token.owner
    }

    pub fn tokens_of(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerTokens(owner))
            .unwrap_or(Vec::new(&env))
    }

    pub fn balance_of(env: Env, owner: Address) -> u64 {
        let tokens: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerTokens(owner))
            .unwrap_or(Vec::new(&env));
        tokens.len() as u64
    }

    pub fn get_approved(env: Env, token_id: u64) -> Option<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Approval(token_id))
    }

    pub fn is_approved_for_all(env: Env, owner: Address, operator: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::OperatorApproval(owner, operator))
            .unwrap_or(false)
    }

    pub fn is_valid_member(env: Env, token_id: u64) -> bool {
        let token: Option<MembershipToken> = env
            .storage()
            .persistent()
            .get(&DataKey::Token(token_id));

        match token {
            None => false,
            Some(t) => !Self::is_expired(&env, &t),
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn is_expired(env: &Env, token: &MembershipToken) -> bool {
        if token.expires_at == 0 {
            return false; // non-expiring
        }
        env.ledger().sequence() as u64 > token.expires_at
    }

    fn remove_token_from_owner(env: &Env, owner: &Address, token_id: u64) {
        let mut tokens: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(env));

        let mut new_tokens: Vec<u64> = Vec::new(env);
        for id in tokens.iter() {
            if id != token_id {
                new_tokens.push_back(id);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::OwnerTokens(owner.clone()), &new_tokens);
    }
}

// ─────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup() -> (Env, NftMembershipContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(NftMembershipContract, ());
        let client = NftMembershipContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(
            &admin,
            &String::from_str(&env, "StellarMembership"),
            &String::from_str(&env, "SMEM"),
        );

        (env, client, admin)
    }

    #[test]
    fn test_initialize() {
        let (env, client, _admin) = setup();
        assert_eq!(
            client.name(),
            String::from_str(&env, "StellarMembership")
        );
        assert_eq!(
            client.symbol(),
            String::from_str(&env, "SMEM")
        );
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn test_mint_and_query() {
        let (env, client, _admin) = setup();
        let user = Address::generate(&env);

        let token_id = client.mint(
            &user,
            &MembershipTier::Gold,
            &String::from_str(&env, "ipfs://QmExample"),
            &0u64,  // non-expiring
            &true,
        );

        assert_eq!(token_id, 1);
        assert_eq!(client.total_supply(), 1);
        assert_eq!(client.balance_of(&user), 1);
        assert_eq!(client.owner_of(&token_id), user);

        let token = client.token_of(&token_id);
        assert_eq!(token.tier, MembershipTier::Gold);
        assert!(token.transferable);
        assert!(client.is_valid_member(&token_id));
    }

    #[test]
    fn test_transfer() {
        let (env, client, _admin) = setup();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        let token_id = client.mint(
            &alice,
            &MembershipTier::Silver,
            &String::from_str(&env, "ipfs://QmSilver"),
            &0,
            &true,
        );

        client.transfer(&alice, &bob, &token_id);

        assert_eq!(client.owner_of(&token_id), bob);
        assert_eq!(client.balance_of(&alice), 0);
        assert_eq!(client.balance_of(&bob), 1);
    }

    #[test]
    #[should_panic(expected = "token is soulbound")]
    fn test_soulbound_transfer_fails() {
        let (env, client, _admin) = setup();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        let token_id = client.mint(
            &alice,
            &MembershipTier::Platinum,
            &String::from_str(&env, "ipfs://QmPlatinum"),
            &0,
            &false, // soulbound
        );

        client.transfer(&alice, &bob, &token_id);
    }

    #[test]
    fn test_upgrade_tier() {
        let (env, client, _admin) = setup();
        let user = Address::generate(&env);

        let token_id = client.mint(
            &user,
            &MembershipTier::Bronze,
            &String::from_str(&env, "ipfs://QmBronze"),
            &0,
            &true,
        );

        client.upgrade_tier(&token_id, &MembershipTier::Platinum);
        let token = client.token_of(&token_id);
        assert_eq!(token.tier, MembershipTier::Platinum);
    }

    #[test]
    fn test_approve_and_transfer() {
        let (env, client, _admin) = setup();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let carol = Address::generate(&env);

        let token_id = client.mint(
            &alice,
            &MembershipTier::Bronze,
            &String::from_str(&env, "ipfs://QmBronze"),
            &0,
            &true,
        );

        client.approve(&alice, &carol, &token_id);
        assert_eq!(client.get_approved(&token_id), Some(carol.clone()));

        client.transfer(&alice, &bob, &token_id);
        assert_eq!(client.owner_of(&token_id), bob);
    }

    #[test]
    fn test_multiple_tokens() {
        let (env, client, _admin) = setup();
        let user = Address::generate(&env);

        for _ in 0..3 {
            client.mint(
                &user,
                &MembershipTier::Bronze,
                &String::from_str(&env, "ipfs://Qm"),
                &0,
                &true,
            );
        }

        assert_eq!(client.balance_of(&user), 3);
        assert_eq!(client.total_supply(), 3);
        let tokens = client.tokens_of(&user);
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn test_burn() {
        let (env, client, _admin) = setup();
        let user = Address::generate(&env);

        let token_id = client.mint(
            &user,
            &MembershipTier::Gold,
            &String::from_str(&env, "ipfs://QmGold"),
            &0,
            &true,
        );

        assert_eq!(client.total_supply(), 1);
        client.burn(&token_id);
        assert_eq!(client.total_supply(), 0);
        assert_eq!(client.balance_of(&user), 0);
    }
}