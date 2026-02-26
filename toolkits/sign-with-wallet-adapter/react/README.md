# Wallet Adapter + Light Token (React)

Wallet Adapter handles wallet connection and transaction signing. You build transactions with light-token instructions and the connected wallet signs them client-side:

1. Connect wallet via Wallet Adapter
2. Build unsigned transaction
3. Sign transaction using connected wallet (Phantom, Backpack, Solflare, etc.)
4. Send signed transaction to RPC

Light Token gives you rent-free token accounts on Solana. Light-token accounts hold balances from any light, SPL, or Token-2022 mint.


## What you will implement

| | SPL | Light Token |
| --- | --- | --- |
| [**Transfer**](#hooks) | `transferChecked()` | `createTransferInterfaceInstruction()` |
| [**Wrap**](#hooks) | N/A | `createWrapInstruction()` |
| [**Get balance**](#hooks) | `getAccount()` | `getAtaInterface()` |
| [**Transaction history**](#hooks) | `getSignaturesForAddress()` | `getSignaturesForOwnerInterface()` |

### Source files

#### Hooks

- **[useTransfer.ts](src/hooks/useTransfer.ts)** — Transfer light-tokens between wallets. Auto-loads cold balance before sending.
- **[useWrap.ts](src/hooks/useWrap.ts)** — Wrap SPL or T22 tokens into light-token associated token account. Auto-detects token program.
- **[useUnwrap.ts](src/hooks/useUnwrap.ts)** — Unwrap light-token associated token account back to SPL or T22. Hook only, not wired into UI.
- **[useLightBalance.ts](src/hooks/useLightBalance.ts)** — Query hot, cold, and unified Light Token balance for a single mint.
- **[useUnifiedBalance.ts](src/hooks/useUnifiedBalance.ts)** — Query balance breakdown: SOL, SPL, Token 2022, light-token hot, and compressed cold.
- **[useTransactionHistory.ts](src/hooks/useTransactionHistory.ts)** — Fetch transaction history for light-token operations.

#### Components

- **[TransferForm.tsx](src/components/sections/TransferForm.tsx)** — Single "Send" button. Routes by token type: light-token -> light-token, or SPL/Token 2022 are wrapped then transfered in one transaction.
- **[TransactionHistory.tsx](src/components/sections/TransactionHistory.tsx)** — Recent light-token interface transactions with explorer links.
- **[WalletInfo.tsx](src/components/sections/WalletInfo.tsx)** — Wallet address display.
- **[TransactionStatus.tsx](src/components/sections/TransactionStatus.tsx)** — Last transaction signature with explorer link.

> Light Token is currently deployed on **devnet**. The interface PDA pattern described here applies to mainnet.

## Before you start

### Your mint needs an SPL interface PDA

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

## Setup

```bash
npm install
cp .env.example .env
# Fill in your credentials
```

### Environment variables

| Variable | Description |
| -------- | ----------- |
| `VITE_HELIUS_RPC_URL` | Helius RPC endpoint (e.g. `https://devnet.helius-rpc.com?api-key=...`). Required for ZK compression indexing. |

### Setup helpers (local keypair)

Setup scripts live in [`scripts/`](../scripts/). They use the Solana CLI keypair at `~/.config/solana/id.json`.

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

```bash
# 1. Create a test mint with interface PDA + fund your wallet
cd ../scripts && npm run mint:spl-and-wrap <your-wallet-address>

# 2. Start the dev server
cd ../react && npm run dev
```

Then in the browser:
1. Connect your wallet via the Wallet Adapter modal
2. Select a light-token balance from the dropdown
3. Enter a recipient address and amount
4. Click "Send" — the app transfers directly
5. Select an SPL balance — the app wraps to light-token then transfers (two signing prompts)

## Tests

```bash
# Unit tests (no network)
pnpm test

# Integration tests (devnet)
VITE_HELIUS_RPC_URL=https://devnet.helius-rpc.com?api-key=... pnpm test:integration
```
