# Light Token SDK - Toolkits

Integration examples for wallets, payments, and streaming.

### Payments and Wallets

The Light-token API matches the SPL-token API almost entirely, and extends their functionality to include the light token program in addition to the SPL-token and Token-2022 programs.
Your users hold and receive tokens of the same mints, just stored more efficiently. Get an overview [in the README](payments-and-wallets/README.md).
- **[get-balance](payments-and-wallets/get-balance.ts)** - Fetch token balances
- **[get-history](payments-and-wallets/get-history.ts)** - Fetch transaction history
- **[send-and-receive](payments-and-wallets/send-and-receive.ts)** - Send and receive light-tokens
- **[wrap](payments-and-wallets/wrap.ts)** - Wrap SPL/T22 to light-token
- **[unwrap](payments-and-wallets/unwrap.ts)** - Unwrap light-token to SPL/T22

### Streaming Tokens

[Rust program example to stream mint events](streaming-tokens/) of the Light-Token Program.

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
