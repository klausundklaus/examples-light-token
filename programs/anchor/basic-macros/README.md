# Light Token macro examples

| | |
|---------|--------|
| [**counter**](counter) | Create PDA with sponsored rent-exemption |
| [**create-associated-token-account**](create-ata) | Create associated light-token account |
| [**create-mint**](create-mint) | Create light-token mint |
| [**create-token-account**](create-token-account) | Create light-token account |

## Combining macros with CPI

[`create-and-transfer`](../create-and-transfer) shows the pattern for instructions that both create accounts and execute logic. The `#[light_account]` macro on `destination` creates the recipient associated token account. The handler body calls `TransferInterfaceCpi` to execute the transfer.

For existing programs, you can replace spl_token with light_token instructions as you need. The API is a superset of SPL-token so switching is straightforward.

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