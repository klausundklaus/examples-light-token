# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Two example apps demonstrating Privy wallet integration with Light Token on Solana devnet. Both apps show transfer, wrap, unwrap, load, and balance query flows using Privy-managed wallets for transaction signing.

## Sub-projects

### `nodejs/` — Server-side (Node.js)

Backend scripts using `@privy-io/node` with server-side wallet signing via `TREASURY_AUTHORIZATION_KEY`. Each script is a standalone operation run with `tsx`.

### `react/` — Client-side (React + Vite) — WIP

Browser app using `@privy-io/react-auth` with client-side wallet signing via `useSignTransaction`. Privy creates embedded Solana wallets on login. Vite dev server with Tailwind CSS v4 and `vite-plugin-node-polyfills` for Buffer.

## Build and run

### Node.js scripts

```bash
cd nodejs
npm install
cp .env.example .env  # fill in all values

# App operations (Privy-signed server wallet)
npm run transfer      # Light-token ATA → ATA transfer (auto-loads cold balance)
npm run wrap          # SPL/T22 → light-token ATA
npm run unwrap        # Light-token ATA → SPL/T22
npm run load          # Consolidate cold + SPL + T22 into light-token ATA
npm run balances      # Query balance breakdown (hot, cold, SPL/T22, SOL)
npm run history       # Transaction history for light-token interface ops

# Setup helpers (use local filesystem keypair, not Privy)
npm run mint:spl-and-wrap       # Create mint + interface PDA + fund treasury
npm run mint:spl               # Mint SPL/T22 tokens to existing mint
npm run register:spl-interface # Register interface PDA on existing mint
npm run decompress             # Decompress light-token ATA to T22 ATA
```

### React app

```bash
cd react
npm install
cp .env.example .env  # fill in VITE_PRIVY_APP_ID and VITE_HELIUS_RPC_URL

npm run dev      # start Vite dev server
npm run build    # production build
```

## Architecture

### Privy signing pattern

Both apps follow the same flow: build an unsigned `Transaction`, serialize with `requireAllSignatures: false`, sign via Privy, deserialize, and send with `sendRawTransaction`.

**Node.js** — signs via `privy.wallets().solana().signTransaction(walletId, { transaction, authorization_context })`. Requires `TREASURY_WALLET_ID` and `TREASURY_AUTHORIZATION_KEY`.

**React** — signs via `useSignTransaction` hook: `signTransaction({ transaction, wallet, chain: 'solana:devnet' })`. Privy handles embedded wallet key management client-side.

### Node.js modules (`nodejs/src/`)

Standalone async functions, each creating their own `PrivyClient` and `createRpc`:

- `transfer.ts` — `createTransferInterfaceInstruction`, auto-loads cold balance via `createLoadAtaInstructionsFromInterface`
- `wrap.ts` — `createWrapInstruction` with SPL interface lookup via `getSplInterfaceInfos`
- `unwrap.ts` — `createUnwrapInstruction` from `@lightprotocol/compressed-token/unified`
- `load.ts` — `createLoadAtaInstructionsFromInterface` to consolidate cold + SPL + T22 into light-token ATA
- `balances.ts` — queries hot (`getAtaInterface`), cold (`getCompressedTokenBalancesByOwnerV2`), SPL T22 (`getTokenAccountsByOwner` + raw data parsing)
- `get-transaction-history.ts` — `getSignaturesForOwnerInterface`
- `config.ts` — centralized env var exports with validation

**Setup helpers** (`nodejs/src/helpers/`):

- `mint-spl-and-wrap.ts` — `createMintInterface` + mint + wrap + transfer to treasury (filesystem wallet)
- `mint-spl.ts` — `createMintToInstruction` to existing mint (filesystem wallet)
- `register-spl-interface.ts` — `createSplInterface` on existing mint (filesystem wallet)
- `decompress.ts` — `decompressInterface` from light-token ATA to T22 ATA (filesystem wallet)

### Transaction routing (React `TransferForm`)

The `TransferForm` component routes actions based on `TokenBalance.isLightToken`:

- Light-token balance → `useTransfer` → `createTransferInterfaceInstruction`
- SPL balance → `useWrap` → `createWrapInstruction` (wraps to own light-token ATA)
- SOL → display only, no transfer action

### React hooks (`react/src/hooks/`)

Each hook returns `{ actionFn, isLoading }` and accepts `{ params, wallet, signTransaction }`:

- `useTransfer` — light-token ATA to ATA transfer
- `useWrap` — SPL to light-token (creates light-token ATA idempotently, verifies SPL balance)
- `useUnwrap` — light-token to SPL T22 (creates T22 ATA if missing)
- `useLightTokenBalances` — fetches SOL, SPL (Token Program), and light-token (T22) balances by parsing raw account data
- `useTransactionHistory` — queries `getSignaturesForOwnerInterface`

## Environment variables

### Node.js (`nodejs/.env`)

| Variable | Required | Purpose |
|---|---|---|
| `PRIVY_APP_ID` | Yes | Privy application ID |
| `PRIVY_APP_SECRET` | Yes | Privy server-side secret |
| `TREASURY_WALLET_ID` | Yes | Privy wallet ID for signing |
| `TREASURY_WALLET_ADDRESS` | Yes | Public key of treasury wallet |
| `TREASURY_AUTHORIZATION_KEY` | Yes | EC private key for ECDSA transaction authorization |
| `HELIUS_RPC_URL` | Yes | Helius RPC endpoint (devnet) |
| `TEST_MINT` | No | Token mint address for scripts |
| `DEFAULT_TEST_RECIPIENT` | No | Defaults to `TREASURY_WALLET_ADDRESS` |
| `DEFAULT_AMOUNT` | No | Defaults to `0.001` |
| `DEFAULT_DECIMALS` | No | Defaults to `9` |

### React (`react/.env`)

| Variable | Required | Purpose |
|---|---|---|
| `VITE_PRIVY_APP_ID` | Yes | Privy application ID |
| `VITE_HELIUS_RPC_URL` | Yes | Helius RPC endpoint (devnet) |

## Key dependencies

- `@privy-io/node` ^0.1.0-alpha.2 — server-side Privy SDK
- `@privy-io/react-auth` ^3.9.1 — client-side Privy SDK
- `@lightprotocol/compressed-token` beta — light-token instructions (also exports `/unified` subpath for unwrap, load, balances)
- `@lightprotocol/stateless.js` beta — RPC client (`createRpc`)
- `@solana/web3.js` 1.98.4 — Solana web3 v1
- `@solana/spl-token` ^0.4.13 — SPL token operations (T22 ATA creation, balance checks)
- `@solana/kit` ^5.5.1 — Solana RPC for Privy provider config (React only)

## Important patterns

- Wrap and unwrap require `ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 })`. Transfer does not.
- Wrap calls `getSplInterfaceInfos` to find the initialized SPL interface and its `tokenProgram`.
- Unwrap imports from `@lightprotocol/compressed-token/unified` (not the main export).
- Helper scripts use filesystem wallet (`~/.config/solana/id.json`), not Privy, because they need a keypair signer for mint authority.
- The React app derives WebSocket URL from RPC URL by replacing `https://` with `wss://`.
- Both apps target `solana:devnet` (CAIP-2 chain ID `solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1`).