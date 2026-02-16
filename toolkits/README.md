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

### Sign with Privy

Light-token operations signed with [Privy](https://privy.io) wallets. Server-side (Node.js) and client-side (React) examples for transfer, wrap, unwrap, load, and balance queries on devnet.
- **[Node.js](sign-with-privy/nodejs/)** — Server-side scripts using `@privy-io/node` with server wallet signing
- **[React](sign-with-privy/react/)** — Browser app using `@privy-io/react-auth` with embedded wallet signing
- **[Setup scripts](sign-with-privy/scripts/)** — Create test mints and fund wallets on devnet

### Streaming Tokens

[Rust program example to stream mint events](streaming-tokens/) of the Light-Token Program.

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
