# Light Token Examples

Light token is a high-performance token standard that reduces the cost of mint and token accounts by 200x.

* All light mint and token accounts are on-chain accounts like SPL, but the light token program sponsors the rent-exemption cost for you.
* Light-token accounts can hold balances from any light, SPL, or Token-2022 mint.
* Light-mint accounts represent a unique mint and optionally can store token-metadata. Functionally equivalent to SPL mints.

## Toolkits

* **[Payments and Wallets](toolkits/payments-and-wallets/)** - All you need for wallet integrations and payment flows. Minimal API differences to SPL.
* **[Streaming Tokens](toolkits/streaming-tokens/)** - Stream mint events using Laserstream

## Client Examples

| Example | Action | Instruction | Description |
|---------|--------|-------------|-------------|
| **create-mint** | [.ts](typescript-client/actions/create-mint.ts) [.rs](rust-client/actions/create_mint.rs) | [.ts](typescript-client/instructions/create-mint.ts) [.rs](rust-client/instructions/create_mint.rs) | Create a light-token mint with metadata |
| **create-ata** | [.ts](typescript-client/actions/create-ata.ts) [.rs](rust-client/actions/create_ata.rs) | [.ts](typescript-client/instructions/create-ata.ts) [.rs](rust-client/instructions/create_ata.rs) | Create an associated light-token account |
| **create-token-account** | — | [.rs](rust-client/instructions/create_token_account.rs) | Create a light-token account with custom owner |
| **load-ata** | [.ts](typescript-client/actions/load-ata.ts) | [.ts](typescript-client/instructions/load-ata.ts) | Load token accounts from light-token, compressed tokens, SPL/T22 to one unified balance |
| **mint-to** | [.ts](typescript-client/actions/mint-to.ts) [.rs](rust-client/actions/mint_to.rs) | [.ts](typescript-client/instructions/mint-to.ts) [.rs](rust-client/instructions/mint_to.rs) | Mint tokens to a light-account |
| **mint-to-checked** | — | [.rs](rust-client/instructions/mint_to_checked.rs) | Mint tokens with decimal validation |
| **transfer-interface** | [.ts](typescript-client/actions/transfer-interface.ts) [.rs](rust-client/actions/transfer_interface.rs) | [.ts](typescript-client/instructions/transfer-interface.ts) [.rs](rust-client/instructions/transfer_interface.rs) | Transfer between light-token, T22, and SPL accounts |
| **transfer-checked** | [.rs](rust-client/actions/transfer_checked.rs) | [.rs](rust-client/instructions/transfer_checked.rs) | Transfer with decimal validation |
| **spl-to-light-transfer** | — | [.rs](rust-client/instructions/spl_to_light_transfer.rs) | Transfer from SPL to Light via TransferInterface |
| **wrap** | [.ts](typescript-client/actions/wrap.ts) [.rs](rust-client/actions/wrap.rs) | [.ts](typescript-client/instructions/wrap.ts) | Wrap SPL/T22 to light-token |
| **unwrap** | [.ts](typescript-client/actions/unwrap.ts) [.rs](rust-client/actions/unwrap.rs) | [.ts](typescript-client/instructions/unwrap.ts) | Unwrap light-token to SPL/T22 |
| **burn** | — | [.rs](rust-client/instructions/burn.rs) | Burn tokens |
| **burn-checked** | — | [.rs](rust-client/instructions/burn_checked.rs) | Burn tokens with decimal validation |
| **approve** | [.ts](typescript-client/actions/delegate-approve.ts) [.rs](rust-client/actions/approve.rs) | [.rs](rust-client/instructions/approve.rs) | Approve delegate |
| **revoke** | [.ts](typescript-client/actions/delegate-revoke.ts) [.rs](rust-client/actions/revoke.rs) | [.rs](rust-client/instructions/revoke.rs) | Revoke delegate |
| **freeze** | — | [.rs](rust-client/instructions/freeze.rs) | Freeze a token account |
| **thaw** | — | [.rs](rust-client/instructions/thaw.rs) | Thaw a frozen token account |
| **close** | — | [.rs](rust-client/instructions/close.rs) | Close a token account |

## Program Examples

### Instructions

The instructions use pure CPI calls which you can combine with existing and / or light macros.
For existing programs, you can replace spl_token with light_token instructions as you need. The API is a superset of SPL-token so switching is straightforward.

| Example | Description |
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

### Macros

| Example | Description |
|---------|-------------|
| [counter](programs/anchor/basic-macros/counter) | Create PDA with sponsored rent-exemption |
| [create-associated-token-account](programs/anchor/basic-macros/create-ata) | Create associated light-token account |
| [create-mint](programs/anchor/basic-macros/create-mint) | Create light-token mint |
| [create-token-account](programs/anchor/basic-macros/create-token-account) | Create light-token account |

### Examples

| Example | Description |
|---------|-------------|
| [create-and-transfer](programs/anchor/create-and-transfer) | Create account via macro and transfer via CPI |


## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).