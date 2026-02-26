# React + Wallet Adapter — Light Token Example

Send, wrap, and unwrap Light Tokens using `@solana/wallet-adapter-react`.

## Setup

```bash
cp .env.example .env
# Fill in VITE_HELIUS_RPC_URL (devnet)

pnpm install
pnpm dev
```

## Tests

```bash
# Unit tests (no network)
pnpm test

# Integration tests (devnet)
VITE_HELIUS_RPC_URL=https://devnet.helius-rpc.com?api-key=... pnpm test:integration
```

## Stack

- React 19, Vite, Tailwind v4
- `@solana/wallet-adapter-react` + `@solana/wallet-adapter-react-ui`
- `@lightprotocol/compressed-token` (unified interface)
- `@solana/web3.js` 1.x
