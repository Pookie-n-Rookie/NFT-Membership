

# 🌟 NFT Membership — Soroban Smart Contract on Stellar

> A fully on-chain NFT membership system built with Soroban (Rust) on the Stellar blockchain. Issue tiered, expirable, and optionally soulbound membership tokens to your community — no backend required.

---

## 📖 Project Description

**NFT Membership** is a Soroban smart contract that lets any organization, protocol, or community manage membership passes as non-fungible tokens directly on the Stellar blockchain. Each membership is a unique, verifiable token tied to a member's wallet address — no database, no centralized authority.

Built with Soroban SDK v22, it follows NFT standards adapted for the Stellar ecosystem, with additional membership-specific features like tiers, expiry, and soulbound (non-transferable) tokens.

---
## 🌐 Deployed Smart Contract
 
| Network | Contract ID |
|---|---|
| **Stellar Testnet** | [`CCEQNLT6GS5QGLYUNW3YGUPQQCSAWUNP4YV3YA757BAJ5NIL3PJY6SCI`](https://stellar.expert/explorer/testnet/contract/CCEQNLT6GS5QGLYUNW3YGUPQQCSAWUNP4YV3YA757BAJ5NIL3PJY6SCI) |
 
> ℹ️ View live contract state, transactions, and events on [Stellar Expert](https://stellar.expert/explorer/testnet).
 
---

## ✨ What It Does

The contract lets an **admin** mint NFT membership tokens to any wallet address. Each token carries:

- A **membership tier** (Bronze → Silver → Gold → Platinum)
- An optional **expiry** (by ledger sequence number)
- A **metadata URI** pointing to off-chain metadata (e.g., IPFS)
- A **transferability flag** — soulbound tokens can never be moved

Members can **transfer** their tokens (if allowed), and the admin can **upgrade tiers**, **extend membership**, or **revoke** (burn) tokens at any time.

---

## 🚀 Features

| Feature | Description |
|---|---|
| 🎫 **NFT Minting** | Admin mints unique membership tokens to any address |
| 🥉🥈🥇💎 **Tiered Membership** | Four tiers: Bronze, Silver, Gold, Platinum |
| ⏳ **Expirable Tokens** | Set expiry via ledger sequence; `0` = never expires |
| 🔒 **Soulbound Support** | Non-transferable tokens for identity-bound memberships |
| 🔁 **Transfers** | Transferable tokens can be sent to other wallets |
| ✅ **Approval System** | Per-token and operator-level approvals (ERC-721 style) |
| ⬆️ **Tier Upgrades** | Admin can upgrade or downgrade a member's tier |
| 🔥 **Burn / Revoke** | Admin can revoke any membership at any time |
| 🔍 **Membership Validation** | On-chain `is_valid_member()` check for integrations |
| 📋 **Wallet Index** | Query all token IDs owned by any address |
| 📊 **Supply Tracking** | Real-time total supply counter |
| 📣 **Events** | All state changes emit Soroban events for indexers |

---

## 🏗️ Project Structure

```
nft-membership/
├── Cargo.toml                          # Workspace root
└── contracts/
    └── nft_membership/
        ├── Cargo.toml                  # Contract dependencies
        └── src/
            └── lib.rs                  # Contract logic + tests
```

---

## 🔧 Contract Interface

### Initialisation

```rust
fn initialize(env, admin: Address, name: String, symbol: String)
```

### Admin Functions

```rust
fn mint(env, to: Address, tier: MembershipTier, metadata_uri: String,
        expires_at: u64, transferable: bool) -> u64
fn burn(env, token_id: u64)
fn upgrade_tier(env, token_id: u64, new_tier: MembershipTier)
fn extend_membership(env, token_id: u64, new_expires_at: u64)
fn transfer_admin(env, new_admin: Address)
```

### Member Functions

```rust
fn transfer(env, from: Address, to: Address, token_id: u64)
fn approve(env, owner: Address, approved: Address, token_id: u64)
fn set_operator_approval(env, owner: Address, operator: Address, approved: bool)
```

### Read-Only Queries

```rust
fn name(env) -> String
fn symbol(env) -> String
fn admin(env) -> Address
fn total_supply(env) -> u64
fn token_of(env, token_id: u64) -> MembershipToken
fn owner_of(env, token_id: u64) -> Address
fn tokens_of(env, owner: Address) -> Vec<u64>
fn balance_of(env, owner: Address) -> u64
fn get_approved(env, token_id: u64) -> Option<Address>
fn is_approved_for_all(env, owner: Address, operator: Address) -> bool
fn is_valid_member(env, token_id: u64) -> bool
```

---

## 🛠️ Getting Started

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli --features opt
```

### Build

```bash
stellar contract build
```

The compiled `.wasm` will be at:
```
target/wasm32-unknown-unknown/release/nft_membership.wasm
```

### Run Tests

```bash
cargo test
```

Expected output:
```
running 7 tests
test tests::test_initialize ... ok
test tests::test_mint_and_query ... ok
test tests::test_transfer ... ok
test tests::test_soulbound_transfer_fails ... ok
test tests::test_upgrade_tier ... ok
test tests::test_multiple_tokens ... ok
test tests::test_burn ... ok

test result: ok. 7 passed; 0 failed
```

### Deploy to Testnet

```bash
# Configure Testnet identity
stellar keys generate --global alice --network testnet
stellar keys fund alice --network testnet

# Deploy
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/nft_membership.wasm \
  --source alice \
  --network testnet

# Initialise
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --name "StellarMembership" \
  --symbol "SMEM"
```

### Mint a Membership Token

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- mint \
  --to <MEMBER_ADDRESS> \
  --tier Gold \
  --metadata_uri "ipfs://QmYourMetadataHash" \
  --expires_at 0 \
  --transferable true
```

---

## 🌐 Deployed Smart Contract

| Network | Contract ID |
|---|---|
| **Stellar Testnet** | [`CCEQNLT6GS5QGLYUNW3YGUPQQCSAWUNP4YV3YA757BAJ5NIL3PJY6SCI`](https://stellar.expert/explorer/testnet/contract/CCEQNLT6GS5QGLYUNW3YGUPQQCSAWUNP4YV3YA757BAJ5NIL3PJY6SCI) |

> ℹ️ View live contract state, transactions, and events on [Stellar Expert](https://stellar.expert/explorer/testnet).

---

## 📐 Data Types

### `MembershipTier`
```rust
pub enum MembershipTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}
```

### `MembershipToken`
```rust
pub struct MembershipToken {
    pub token_id: u64,
    pub owner: Address,
    pub tier: MembershipTier,
    pub issued_at: u64,       // ledger sequence at mint
    pub expires_at: u64,      // 0 = never expires
    pub metadata_uri: String, // IPFS / Arweave URI
    pub transferable: bool,   // false = soulbound
}
```

---

## 🔐 Security

- All state-mutating functions require `require_auth()` on the relevant signer
- Admin-only operations enforce admin auth before any state change
- Soulbound tokens revert on any transfer attempt
- Expired tokens are rejected during transfers
- Contract can only be initialized once (re-init panics)

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

---

## 🤝 Contributing

PRs welcome! Open an issue first for major changes.

---

*Built with ❤️ on [Stellar](https://stellar.org) using [Soroban](https://soroban.stellar.org)*