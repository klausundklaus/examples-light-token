# Light Token - Rust Client

Rust client examples for light-token and light-token-client.

- **create-mint** - Create a light-token mint with metadata
  - [Action](actions/create_mint.rs) | [Instruction](instructions/create_mint.rs)
- **create-associated-token-account** - Create an associated light-token account
  - [Action](actions/create_associated_token_account.rs) | [Instruction](instructions/create_associated_token_account.rs)
- **create-token-account** - Create a light-token account with custom owner
  - [Instruction](instructions/create_token_account.rs)
- **mint-to** - Mint tokens to a light-account
  - [Action](actions/mint_to.rs) | [Instruction](instructions/mint_to.rs)
- **mint-to-checked** - Mint tokens with decimal validation
  - [Instruction](instructions/mint_to_checked.rs)
- **transfer-interface** - Transfer between light-token, T22, and SPL accounts
  - [Action](actions/transfer_interface.rs) | [Instruction](instructions/transfer_interface.rs)
- **spl-to-light-transfer** - Transfer from SPL to Light via TransferInterface
  - [Instruction](instructions/spl_to_light_transfer.rs)
- **wrap** - Wrap SPL/T22 to light-token
  - [Action](actions/wrap.rs)
- **unwrap** - Unwrap light-token to SPL/T22
  - [Action](actions/unwrap.rs)
- **burn** - Burn tokens
  - [Instruction](instructions/burn.rs)
- **burn-checked** - Burn tokens with decimal validation
  - [Instruction](instructions/burn_checked.rs)
- **approve** - Approve delegate
  - [Action](actions/approve.rs) | [Instruction](instructions/approve.rs)
- **revoke** - Revoke delegate
  - [Action](actions/revoke.rs) | [Instruction](instructions/revoke.rs)
- **freeze** - Freeze a token account
  - [Instruction](instructions/freeze.rs)
- **thaw** - Thaw a frozen token account
  - [Instruction](instructions/thaw.rs)
- **close** - Close a token account
  - [Instruction](instructions/close.rs)

## Run

```bash
cargo run --example action_create_mint
cargo run --example instruction_mint_to_checked
```

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
