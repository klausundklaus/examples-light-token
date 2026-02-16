# Light Token Examples

Light token is a high-performance token standard that reduces the cost of mint and token accounts by 200x.

* All light mint and token accounts are on-chain accounts like SPL, but the light token program sponsors the rent-exemption cost for you.
* Light-token accounts can hold balances from any light, SPL, or Token-2022 mint.
* Light-mint accounts represent a unique mint and optionally can store token-metadata. Functionally equivalent to SPL mints.

## Toolkits

|  | Description |
|---------|-------------|
| [Payments and Wallets](toolkits/payments-and-wallets/) | All you need for wallet integrations and payment flows. Minimal API differences to SPL. |
| [Streaming Tokens](toolkits/streaming-tokens/) | Stream mint events using Laserstream |
| [Sign with Privy](toolkits/sign-with-privy/) | Light-token operations signed with Privy wallets (Node.js + React) |

## Client Examples

### TypeScript

|  |  |  | Description |
|---------|--------|-------------|-------------|
| create-mint | [Action](typescript-client/actions/create-mint.ts) | [Instruction](typescript-client/instructions/create-mint.ts) | Create a light-token mint with metadata |
| create-ata | [Action](typescript-client/actions/create-ata.ts) | [Instruction](typescript-client/instructions/create-ata.ts) | Create an associated light-token account |
| load-ata | [Action](typescript-client/actions/load-ata.ts) | [Instruction](typescript-client/instructions/load-ata.ts) | Load token accounts from light-token, compressed tokens, SPL/T22 to one unified balance |
| mint-to | [Action](typescript-client/actions/mint-to.ts) | [Instruction](typescript-client/instructions/mint-to.ts) | Mint tokens to a light-account |
| transfer-interface | [Action](typescript-client/actions/transfer-interface.ts) | [Instruction](typescript-client/instructions/transfer-interface.ts) | Transfer between light-token, T22, and SPL accounts |
| wrap | [Action](typescript-client/actions/wrap.ts) | [Instruction](typescript-client/instructions/wrap.ts) | Wrap SPL/T22 to light-token |
| unwrap | [Action](typescript-client/actions/unwrap.ts) | [Instruction](typescript-client/instructions/unwrap.ts) | Unwrap light-token to SPL/T22 |
| approve | [Action](typescript-client/actions/delegate-approve.ts) | | Approve delegate |
| revoke | [Action](typescript-client/actions/delegate-revoke.ts) | | Revoke delegate |

### Rust

|  |  |  | Description |
|---------|--------|-------------|-------------|
| create-mint | [Action](rust-client/actions/create_mint.rs) | [Instruction](rust-client/instructions/create_mint.rs) | Create a light-token mint with metadata |
| create-ata | [Action](rust-client/actions/create_ata.rs) | [Instruction](rust-client/instructions/create_ata.rs) | Create an associated light-token account |
| create-token-account | | [Instruction](rust-client/instructions/create_token_account.rs) | Create a light-token account with custom owner |
| mint-to | [Action](rust-client/actions/mint_to.rs) | [Instruction](rust-client/instructions/mint_to.rs) | Mint tokens to a light-account |
| mint-to-checked | | [Instruction](rust-client/instructions/mint_to_checked.rs) | Mint tokens with decimal validation |
| transfer-interface | [Action](rust-client/actions/transfer_interface.rs) | [Instruction](rust-client/instructions/transfer_interface.rs) | Transfer between light-token, T22, and SPL accounts |
| transfer-checked | [Action](rust-client/actions/transfer_checked.rs) | [Instruction](rust-client/instructions/transfer_checked.rs) | Transfer with decimal validation |
| spl-to-light-transfer | | [Instruction](rust-client/instructions/spl_to_light_transfer.rs) | Transfer from SPL to Light via TransferInterface |
| wrap | [Action](rust-client/actions/wrap.rs) | | Wrap SPL/T22 to light-token |
| unwrap | [Action](rust-client/actions/unwrap.rs) | | Unwrap light-token to SPL/T22 |
| burn | | [Instruction](rust-client/instructions/burn.rs) | Burn tokens |
| burn-checked | | [Instruction](rust-client/instructions/burn_checked.rs) | Burn tokens with decimal validation |
| approve | [Action](rust-client/actions/approve.rs) | [Instruction](rust-client/instructions/approve.rs) | Approve delegate |
| revoke | [Action](rust-client/actions/revoke.rs) | [Instruction](rust-client/instructions/revoke.rs) | Revoke delegate |
| freeze | | [Instruction](rust-client/instructions/freeze.rs) | Freeze a token account |
| thaw | | [Instruction](rust-client/instructions/thaw.rs) | Thaw a frozen token account |
| close | | [Instruction](rust-client/instructions/close.rs) | Close a token account |

## Program Examples

### Examples 

|  | Description |
|---------|-------------|
| [cp-swap-reference](https://github.com/Lightprotocol/cp-swap-reference/tree/954f679699dbdfe9711308c8ae9fbd21e69db8aa) | Fork of Raydium AMM that creates markets without paying rent-exemption. |
| [create-and-transfer](programs/anchor/create-and-transfer) | Create account via macro and transfer via CPI |

### Macros

|  | Description |
|---------|-------------|
| [counter](programs/anchor/basic-macros/counter) | Create, increment and close counter PDA with sponsored rent-exemption |
| [create-associated-token-account](programs/anchor/basic-macros/create-ata) | Create associated light-token account |
| [create-mint](programs/anchor/basic-macros/create-mint) | Create light-token mint |
| [create-token-account](programs/anchor/basic-macros/create-token-account) | Create light-token account |

### Instructions

The instructions use pure CPI calls which you can combine with existing and / or light macros.
For existing programs, you can replace spl_token with light_token instructions as you need. The API is a superset of SPL-token so switching is straightforward.

|  | Description |
|---------|-------------|
| [approve](programs/anchor/basic-instructions/approve/src/lib.rs) | Approve delegate via CPI |
| [burn](programs/anchor/basic-instructions/burn/src/lib.rs) | Burn tokens via CPI |
| [close](programs/anchor/basic-instructions/close/src/lib.rs) | Close token account via CPI |
| [create-associated-token-account](programs/anchor/basic-instructions/create-ata/src/lib.rs) | Create associated light-token account via CPI |
| [create-mint](programs/anchor/basic-instructions/create-mint/src/lib.rs) | Create light-token mint via CPI |
| [create-token-account](programs/anchor/basic-instructions/create-token-account/src/lib.rs) | Create light-token account via CPI |
| [freeze](programs/anchor/basic-instructions/freeze/src/lib.rs) | Freeze token account via CPI |
| [mint-to](programs/anchor/basic-instructions/mint-to/src/lib.rs) | Mint tokens via CPI |
| [revoke](programs/anchor/basic-instructions/revoke/src/lib.rs) | Revoke delegate via CPI |
| [thaw](programs/anchor/basic-instructions/thaw/src/lib.rs) | Thaw token account via CPI |
| [transfer-checked](programs/anchor/basic-instructions/transfer-checked/src/lib.rs) | Transfer with mint validation via CPI |
| [transfer-interface](programs/anchor/basic-instructions/transfer-interface/src/lib.rs) | Transfer between light-token, T22, and SPL accounts via CPI |

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
