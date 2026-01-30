# Light Token Examples

Light token is a high-performance token standard that reduces the cost of mint and token accounts by 200x.

* All light mint and token accounts are on-chain accounts like SPL, but the light token program sponsors the rent-exemption cost for you.
* Light-token accounts can hold balances from any light, SPL, or Token-2022 mint.
* Light-mint accounts represent a unique mint and optionally can store token-metadata. Functionally equivalent to SPL mints.

## Toolkits

* **[Payments and Wallets](toolkits/payments-and-wallets/)** - All you need for wallet integrations and payment flows. Minimal API differences to SPL.
* **[Streaming Tokens](toolkits/streaming-tokens/)** - Stream mint events using Laserstream

## TypeScript Client

* **create-mint** - Create a light-token mint with metadata
  * [Action](typescript-client/actions/create-mint.ts) | [Instruction](typescript-client/instructions/create-mint.ts)
* **create-ata** - Create an associated light-token account
  * [Action](typescript-client/actions/create-ata.ts) | [Instruction](typescript-client/instructions/create-ata.ts)
* **load-ata** - Load token accounts from light-token, compressed tokens, SPL/T22 to one unified balance
  * [Action](typescript-client/actions/load-ata.ts) | [Instruction](typescript-client/instructions/load-ata.ts)
* **mint-to** - Mint tokens to a light-account
  * [Action](typescript-client/actions/mint-to.ts) | [Instruction](typescript-client/instructions/mint-to.ts)
* **transfer-interface** - Transfer between light-token, T22, and SPL accounts
  * [Action](typescript-client/actions/transfer-interface.ts) | [Instruction](typescript-client/instructions/transfer-interface.ts)
* **wrap** - Wrap SPL/T22 to light-token
  * [Action](typescript-client/actions/wrap.ts) | [Instruction](typescript-client/instructions/wrap.ts)
* **unwrap** - Unwrap light-token to SPL/T22
  * [Action](typescript-client/actions/unwrap.ts) | [Instruction](typescript-client/instructions/unwrap.ts)
* **delegate-approve** - Approve delegate
  * [Action](typescript-client/actions/delegate-approve.ts)
* **delegate-revoke** - Revoke delegate
  * [Action](typescript-client/actions/delegate-revoke.ts)

## Rust Client

* **create-mint** - Create a light-token mint with metadata
  * [Action](rust-client/actions/create_mint.rs) | [Instruction](rust-client/instructions/create_mint.rs)
* **create-ata** - Create an associated light-token account
  * [Action](rust-client/actions/create_ata.rs) | [Instruction](rust-client/instructions/create_ata.rs)
* **create-token-account** - Create a light-token account with custom owner
  * [Instruction](rust-client/instructions/create_token_account.rs)
* **mint-to** - Mint tokens to a light-account
  * [Action](rust-client/actions/mint_to.rs) | [Instruction](rust-client/instructions/mint_to.rs)
* **mint-to-checked** - Mint tokens with decimal validation
  * [Instruction](rust-client/instructions/mint_to_checked.rs)
* **transfer-interface** - Transfer between light-token, T22, and SPL accounts
  * [Action](rust-client/actions/transfer_interface.rs) | [Instruction](rust-client/instructions/transfer_interface.rs)
* **transfer-checked** - Transfer with decimal validation
  * [Action](rust-client/actions/transfer_checked.rs) | [Instruction](rust-client/instructions/transfer_checked.rs)
* **spl-to-light-transfer** - Transfer from SPL to Light via TransferInterface
  * [Instruction](rust-client/instructions/spl_to_light_transfer.rs)
* **wrap** - Wrap SPL/T22 to light-token
  * [Action](rust-client/actions/wrap.rs)
* **unwrap** - Unwrap light-token to SPL/T22
  * [Action](rust-client/actions/unwrap.rs)
* **burn** - Burn tokens
  * [Instruction](rust-client/instructions/burn.rs)
* **burn-checked** - Burn tokens with decimal validation
  * [Instruction](rust-client/instructions/burn_checked.rs)
* **approve** - Approve delegate
  * [Action](rust-client/actions/approve.rs) | [Instruction](rust-client/instructions/approve.rs)
* **revoke** - Revoke delegate
  * [Action](rust-client/actions/revoke.rs) | [Instruction](rust-client/instructions/revoke.rs)
* **freeze** - Freeze a token account
  * [Instruction](rust-client/instructions/freeze.rs)
* **thaw** - Thaw a frozen token account
  * [Instruction](rust-client/instructions/thaw.rs)
* **close** - Close a token account
  * [Instruction](rust-client/instructions/close.rs)

## Run

```bash
# TypeScript
cd typescript-client
npm run create-mint:action
npm run mint-to:action
# See package.json for all scripts

# Rust
cd rust-client
cargo run --example action_create_mint
cargo run --example instruction_mint_to_checked
```

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).