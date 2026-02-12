# Light Token Anchor programs

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

## Build and test

```bash
# for localnet
npm i -g @lightprotocol/zk-compression-cli@alpha
```

```bash
anchor build
```

```bash
cargo test-sbf
```

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
