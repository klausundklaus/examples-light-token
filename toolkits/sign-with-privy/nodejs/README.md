# Privy + Light Token (Node.js)

Privy handles user authentication and wallet management. You build transactions with light-token instructions and Privy signs them server-side:

1. Authenticate with Privy
2. Build unsigned transaction
3. Sign transaction using Privy's wallet API
4. Send signed transaction to RPC

Light Token gives you rent-free token accounts on Solana. Light-token accounts hold balances from any light, SPL, or Token-2022 mint.

## What you will implement

| | SPL | Light Token |
| --- | --- | --- |
| [**Transfer**](#operations) | `transferChecked()` | `createTransferInterfaceInstruction()` |
| [**Wrap**](#operations) | N/A | `createWrapInstruction()` |
| [**Unwrap**](#operations) | N/A | `createUnwrapInstruction()` |
| [**Load**](#operations) | N/A | `createLoadAtaInstructionsFromInterface()` |
| [**Get balance**](#operations) | `getAccount()` | `getAtaInterface()` |
| [**Transaction history**](#operations) | `getSignaturesForAddress()` | `getSignaturesForOwnerInterface()` |

### Source files

- **[transfer.ts](src/transfer.ts)** — Transfer light-tokens between wallets. Auto-loads cold balance before sending.
- **[wrap.ts](src/wrap.ts)** — Wrap SPL or T22 tokens into light-token associated token account.
- **[unwrap.ts](src/unwrap.ts)** — Unwrap light-token associated token account back to SPL or T22.
- **[load.ts](src/load.ts)** — Consolidate cold (compressed) and SPL/T22 balances into the light-token associated token account.
- **[light-balance.ts](src/light-balance.ts)** — Query hot, cold, and unified Light Token balance for a single mint.
- **[balances.ts](src/balances.ts)** — Query balance breakdown: hot, cold, SPL/T22, and SOL.
- **[get-transaction-history.ts](src/get-transaction-history.ts)** — Fetch transaction history for light-token operations.

> Light Token is currently deployed on **devnet**. The interface PDA pattern described here applies to mainnet.

## Before you start

### 1. Your mint needs an SPL interface PDA

The interface PDA enables interoperability between SPL/T22 and light-token. It holds SPL/T22 tokens when they're wrapped into light-token format.

**Check if your mint has one:**

```typescript
import { getSplInterfaceInfos } from "@lightprotocol/compressed-token";

const infos = await getSplInterfaceInfos(rpc, mint);
const hasInterface = infos.some((info) => info.isInitialized);
```

**Register one if it doesn't:**

```bash
# For an existing SPL or T22 mint (from scripts/)
cd ../scripts && npm run register:spl-interface <mint-address>
```

Or in code via `createSplInterface(rpc, payer, mint)`. Works with both SPL Token and Token-2022 mints.

**Example: wrapping devnet USDC.** If you have devnet USDC (`4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`), register its interface PDA first, then wrap it into a light-token associated token account. Set `TEST_MINT` in `.env` to the USDC mint address.

### 2. Recipients need balance visibility

Your app must query balances through a ZK compression-compatible RPC (Helius or Triton):

```typescript
import { createRpc } from "@lightprotocol/stateless.js";
import { getAtaInterface } from "@lightprotocol/compressed-token/unified";

const rpc = createRpc(process.env.HELIUS_RPC_URL);

// Hot balance (light-token associated token account)
const { parsed, isCold } = await getAtaInterface(rpc, ata, owner, mint);

// Cold balance (compressed tokens not yet loaded into associated token account)
const compressed = await rpc.getCompressedTokenBalancesByOwnerV2(owner, { mint });
```

See `src/balances.ts` for the full pattern that queries hot, cold, and SPL/T22 balances.

## Setup

```bash
npm install
cp .env.example .env
# Fill in your credentials
```

### Environment variables

| Variable | Description |
| -------- | ----------- |
| `PRIVY_APP_ID` | From the [Privy console](https://console.privy.io). |
| `PRIVY_APP_SECRET` | Server-side secret from Privy console. |
| `TREASURY_WALLET_ID` | UUID of your Privy server wallet. |
| `TREASURY_WALLET_ADDRESS` | Solana public key of that wallet. |
| `TREASURY_AUTHORIZATION_KEY` | EC private key for ECDSA transaction authorization. |
| `HELIUS_RPC_URL` | Helius RPC endpoint (e.g. `https://devnet.helius-rpc.com?api-key=...`). Required for ZK compression indexing. |
| `TEST_MINT` | Mint address to use in example scripts. |

## Operations

### App operations (Privy-signed)

These are the operations your app calls at runtime. Transactions are signed server-side via the Privy API.

| Command | What it does |
| ------- | ----------- |
| `npm run transfer` | Transfer light-tokens between wallets. Auto-loads cold balance before sending. |
| `npm run wrap` | Wrap SPL or T22 tokens into light-token associated token account. Auto-detects token program from the mint's interface PDA. |
| `npm run unwrap` | Unwrap light-token associated token account back to SPL or T22. Auto-detects token program. |
| `npm run load` | Consolidate cold (compressed) and SPL/T22 balances into the light-token associated token account. |
| `npm run light-balance` | Query Light Token balance (hot, cold, unified) for `TEST_MINT`. |
| `npm run balances` | Query balance breakdown: hot, cold, SPL/T22, and SOL. |
| `npm run history` | Fetch transaction history for light-token interface operations. |

### Setup helpers (local keypair)

Setup scripts live in [`scripts/`](../scripts/). They use the Solana CLI keypair at `~/.config/solana/id.json` and don't require Privy credentials.

```bash
cd ../scripts
cp .env.example .env  # set HELIUS_RPC_URL
```

| Command | What it does |
| ------- | ----------- |
| `npm run mint:spl-and-wrap <recipient> [amount] [decimals]` | Create an SPL or T22 mint with interface PDA, mint tokens, wrap, and transfer to recipient. |
| `npm run mint:spl <mint> <recipient> [amount] [decimals]` | Mint additional SPL or T22 tokens to an existing mint. |
| `npm run register:spl-interface <mint>` | Register an interface PDA on an existing SPL or T22 mint. Required for wrap/unwrap. |

## Quick start

Run these in order to see the full flow on devnet:

```bash
# 1. Create a test mint with interface PDA + fund the treasury wallet
cd ../scripts && npm run mint:spl-and-wrap <treasury-wallet-address>

# 2. Check balances — should show 100 tokens in hot balance
cd ../nodejs && npm run balances

# 3. Transfer light-tokens to the default recipient
npm run transfer

# 4. Unwrap light-tokens back to SPL/T22
npm run unwrap

# 5. Wrap SPL/T22 tokens back into light-token associated token account
npm run wrap
```
