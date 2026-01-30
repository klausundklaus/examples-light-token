# Light Token Anchor programs

## Instructions

* The instructions use pure CPI calls which you can combine with existing and / or light macros.
* For existing programs, you can replace spl_token with light_token instructions as you need. The API is a superset of SPL-token so switching is straightforward.

* **[approve](basic-instructions/approve/src/lib.rs)** - Approve delegate via CPI
* **[burn](basic-instructions/burn/src/lib.rs)** - Burn tokens via CPI
* **[close](basic-instructions/close/src/lib.rs)** - Close token account via CPI
* **[create-associated-token-account](basic-instructions/create-ata/src/lib.rs)** - Create associated light-token account via CPI
* **[create-mint](basic-instructions/create-mint/src/lib.rs)** - Create light-token mint via CPI
* **[create-token-account](basic-instructions/create-token-account/src/lib.rs)** - Create light-token account via CPI
* **[freeze](basic-instructions/freeze/src/lib.rs)** - Freeze token account via CPI
* **[mint-to](basic-instructions/mint-to/src/lib.rs)** - Mint tokens via CPI
* **[revoke](basic-instructions/revoke/src/lib.rs)** - Revoke delegate via CPI
* **[thaw](basic-instructions/thaw/src/lib.rs)** - Thaw token account via CPI
* **[transfer-checked](basic-instructions/transfer-checked/src/lib.rs)** - Transfer with mint validation via CPI
* **[transfer-interface](basic-instructions/transfer-interface/src/lib.rs)** - Transfer between light-token, T22, and SPL accounts via CPI

## Macros

* **[counter](basic-macros/counter)** - Create PDA with sponsored rent-exemption
* **[create-associated-token-account](basic-macros/create-ata)** - Create associated light-token account
* **[create-mint](basic-macros/create-mint)** - Create light-token mint
* **[create-token-account](basic-macros/create-token-account)** - Create light-token account

## Composite

* **[create-and-transfer](create-and-transfer)** - Create account via macro and transfer via CPI

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
