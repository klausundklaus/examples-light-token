# Light Token Anchor Solana Programs

- **[escrow](escrow/src/lib.rs)** - Peer-to-peer light-token swap with offer/accept flow
- **[fundraiser](fundraiser/src/lib.rs)** - Token Fundraiser with target, deadline, and refunds
- **[light-token-minter](light-token-minter/src/lib.rs)** - Create light-mints with metadata, mint tokens
- **[token-swap](token-swap/src/lib.rs)** - AMM with liquidity pools and swaps

## Basic Macros

- [**counter**](basic-macros/counter/src/lib.rs) - Minimal Light-PDA example
- [**create-mint**](basic-macros/create-mint/src/lib.rs) - Create a light-token mint
- [**create-associated-token-account**](basic-macros/create-associated-token-account/src/lib.rs) - Create an associated light-token account
- [**create-token-account**](basic-macros/create-token-account/src/lib.rs) - Create a light-token account
- [**create-and-transfer**](create-and-transfer/src/lib.rs) - Create a light-token account and transfer in one instruction

## Basic Instructions

- [**create-mint**](basic-instructions/create-mint/src/lib.rs) - Create a light-token mint via CPI
- [**create-associated-token-account**](basic-instructions/create-associated-token-account/src/lib.rs) - Create an associated light-token account via CPI
- [**create-token-account**](basic-instructions/create-token-account/src/lib.rs) - Create a light-token account via CPI
- [**mint-to**](basic-instructions/mint-to/src/lib.rs) - Mint tokens via CPI
- [**transfer-interface**](basic-instructions/transfer-interface/src/lib.rs) - Transfer between light-token, T22, and SPL accounts via CPI
- [**transfer-checked**](basic-instructions/transfer-checked/src/lib.rs) - Transfer with decimal validation against mint via CPI
- [**approve**](basic-instructions/approve/src/lib.rs) - Approve delegate via CPI
- [**revoke**](basic-instructions/revoke/src/lib.rs) - Revoke delegate via CPI
- [**burn**](basic-instructions/burn/src/lib.rs) - Burn tokens via CPI
- [**freeze**](basic-instructions/freeze/src/lib.rs) - Freeze token account via CPI
- [**thaw**](basic-instructions/thaw/src/lib.rs) - Thaw token account via CPI
- [**close**](basic-instructions/close/src/lib.rs) - Close token account via CPI

## Test

### Requirements

- light cli (install via `npm i -g @lightprotocol/zk-compression-cli@beta`)
- Solana CLI 2.2.15+
- Anchor 0.31.1+
- Rust 1.90.0+

### Build

```bash
cargo build-sbf --manifest-path escrow/Cargo.toml
cargo build-sbf --manifest-path fundraiser/Cargo.toml
cargo build-sbf --manifest-path token-swap/Cargo.toml
cargo build-sbf --manifest-path light-token-minter/Cargo.toml
```

### Run tests

```bash
# Example programs
cargo test-sbf -p escrow -- --test-threads=1
cargo test-sbf -p fundraiser -- --test-threads=1
cargo test-sbf -p light-token-minter -- --test-threads=1
cargo test-sbf -p swap_example -- --test-threads=1

# Basic macros
cargo test-sbf -p counter -- --test-threads=1
cargo test-sbf -p light-token-macro-create-associated-token-account -- --test-threads=1
cargo test-sbf -p light-token-macro-create-mint -- --test-threads=1
cargo test-sbf -p light-token-macro-create-token-account -- --test-threads=1
cargo test-sbf -p create-and-transfer -- --test-threads=1

# Basic instructions
cargo test-sbf -p light-token-anchor-approve -- --test-threads=1
cargo test-sbf -p light-token-anchor-burn -- --test-threads=1
cargo test-sbf -p light-token-anchor-close -- --test-threads=1
cargo test-sbf -p light-token-anchor-create-associated-token-account -- --test-threads=1
cargo test-sbf -p light-token-anchor-create-mint -- --test-threads=1
cargo test-sbf -p light-token-anchor-create-token-account -- --test-threads=1
cargo test-sbf -p light-token-anchor-freeze -- --test-threads=1
cargo test-sbf -p light-token-anchor-mint-to -- --test-threads=1
cargo test-sbf -p light-token-anchor-revoke -- --test-threads=1
cargo test-sbf -p light-token-anchor-thaw -- --test-threads=1
cargo test-sbf -p light-token-anchor-transfer-checked -- --test-threads=1
cargo test-sbf -p light-token-anchor-transfer-interface -- --test-threads=1
```

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).

API is in Beta and subject to change.
Questions or need hands-on support? Join the Developer [Discord](https://discord.com/invite/7cJ8BhAXhu).