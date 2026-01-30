# Light Token Instructions

You can replace spl_token with light_token instructions as you need. The API is a superset of SPL-token so switching is straightforward.

- **[approve](approve/src/lib.rs)** - Approve delegate
- **[burn](burn/src/lib.rs)** - Burn tokens
- **[close](close/src/lib.rs)** - Close token account
- **[create-associated-token-account](create-ata/src/lib.rs)** - Create associated light-token account
- **[create-mint](create-mint/src/lib.rs)** - Create light-token mint
- **[create-token-account](create-token-account/src/lib.rs)** - Create light-token account
- **[freeze](freeze/src/lib.rs)** - Freeze token account
- **[mint-to](mint-to/src/lib.rs)** - Mint tokens
- **[revoke](revoke/src/lib.rs)** - Revoke delegate
- **[thaw](thaw/src/lib.rs)** - Thaw token account
- **[transfer-checked](transfer-checked/src/lib.rs)** - Transfer with mint validation
- **[transfer-interface](transfer-interface/src/lib.rs)** - Transfer between light-token, T22, and SPL accounts

## Macros

The examples use pure CPI calls which you can combine with existing and / or [light macros](../basic-macros).

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