# Setup Scripts

Standalone setup scripts for creating test mints and funding wallets on devnet. These use a local Solana keypair (`~/.config/solana/id.json`) and don't require Privy credentials.

Shared by both the [Node.js](../nodejs/) and [React](../react/) examples.

## Setup

```bash
npm install
cp .env.example .env
# Set HELIUS_RPC_URL
```

## Scripts

| Command | What it does |
| ------- | ----------- |
| `npm run mint:spl-and-wrap <recipient> [amount] [decimals]` | Create T22 mint with interface PDA, mint tokens, wrap, and transfer to recipient. Defaults: 100 tokens, 9 decimals. |
| `npm run mint:spl <mint> <recipient> [amount] [decimals]` | Mint SPL/T22 tokens to an existing mint. Defaults: 100 tokens, 9 decimals. |
| `npm run register:spl-interface <mint>` | Register an interface PDA on an existing mint. Required for wrap/unwrap. |

All scripts accept CLI arguments or fall back to env vars (`RECIPIENT_ADDRESS`, `TEST_MINT`).

## Example: set up a test token for the React app

```bash
# 1. Create a funded light-token mint and send tokens to your Privy wallet
npm run mint:spl-and-wrap <your-privy-wallet-address> 100 9

# 2. Note the mint address from the output, then start the React app
cd ../react
VITE_HELIUS_RPC_URL=... npm run dev
```
