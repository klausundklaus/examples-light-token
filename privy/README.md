# Privy + Light Token

Transfer, wrap, unwrap, and query light-tokens signed with Privy wallets. Learn more in the README of the respective examples.

- **[Node.js](nodejs/)** — Server-side scripts using `@privy-io/node` with server wallet signing
- **[React](react/)** — Browser app using `@privy-io/react-auth` with embedded wallet signing
- **[Setup scripts](scripts/)** — Create test mints and fund wallets on devnet

| Creation cost     | SPL                 | Light Token          |
| :---------------- | :------------------ | :------------------- |
| **Token account** | ~2,000,000 lamports | ~**11,000** lamports |

Privy handles user authentication and wallet management. You build transactions with light-token and Privy signs them:

1. Authenticate with Privy
2. Build unsigned transaction with light-token instructions
3. Sign transaction using Privy's wallet provider
4. Send signed transaction to RPC

## Operations

| | SPL | Light Token |
| --- | --- | --- |
| **Transfer** | `createTransferInstruction()` | `createTransferInterfaceInstructions()` |
| **Receive / Load** | `getOrCreateAssociatedTokenAccount()` | `createLoadAtaInstructions()` |
| **Wrap (SPL → Light)** | N/A | `createWrapInstruction()` |
| **Unwrap (Light → SPL)** | N/A | `createUnwrapInstructions()` |
| **Get balance** | `getAccount()` | `getAtaInterface()` |
| **Transaction history** | `getSignaturesForAddress()` | `getSignaturesForOwnerInterface()` |

### Transfer and Loading Balance

Light Token accounts exist in two states: **hot** (active on-chain with rent-exempt balance) and **cold** (compressed after extended inactivity, `is_initialized: false`). Programs interact only with hot accounts.

`loadAta` reinstates a cold account back to active on-chain state. It unifies balances from compressed tokens, SPL, and Token 2022 into a single Light Token associated token account. Creates the ATA if it doesn't exist. Returns `null` if there's nothing to load (idempotent).

`transfer` and `unwrap` auto-load before executing — explicit `loadAta` is only needed when receiving payments.

APIs return `TransactionInstruction[][]` — each inner array is one transaction. Almost always one. The same loop handles multi-transaction cases.

## Documentation

- [Light Token with Privy wallets](https://www.zkcompression.com/light-token/toolkits/for-privy)
- [Toolkit for stablecoin payments](https://www.zkcompression.com/light-token/toolkits/for-payments)
- [Light Token overview](https://www.zkcompression.com/light-token/welcome)
