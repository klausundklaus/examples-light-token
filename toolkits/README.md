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

### Sign with Wallet Adapter

Sign light-token transactions with [Wallet Adapter](https://github.com/anza-xyz/wallet-adapter). Transfer, wrap, unwrap, and balance queries.
- **[React](sign-with-wallet-adapter/react/)** — Browser app using `@solana/wallet-adapter-react` with Phantom, Backpack, Solflare, etc.

### Streaming Tokens

[Rust program example to stream mint events](streaming-tokens/) of the Light-Token Program.

### Sponsor Rent Top-Ups

Sponsor rent top-ups for users by setting your application as the fee payer.
- **[TypeScript](sponsor-rent-top-ups/typescript/)** - Sponsored Light transfer
- **[Rust](sponsor-rent-top-ups/rust/)** - Sponsored Light transfer

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
