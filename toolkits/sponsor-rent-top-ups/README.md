# Sponsor Rent Top-Ups

Light Token sponsors rent-exemption for Solana accounts and keeps them active through periodic top-ups paid by the fee payer. Set your application as the fee payer to abstract away holding SOL from your users — a rent-free experience similar to transaction fee sponsorship.

- **[TypeScript](typescript/)** — Sponsored transfer using `@lightprotocol/compressed-token`
- **[Rust](rust/)** — Sponsored transfer using `light-token`

Set the `feePayer` field to the public key of the account that will pay the top-ups. Any account that signs the transaction can be the fee payer.

1. Build the transfer instruction with your server as the `feePayer`
2. User signs to authorize the transfer (no SOL needed)
3. Fee payer covers rent top-ups when the transaction lands

> You can set the fee payer to any signing account on any transaction with Light Token.

## Source files

- **[sponsor-top-ups.ts](typescript/sponsor-top-ups.ts)** / **[sponsor-top-ups.rs](rust/sponsor-top-ups.rs)** — Sponsored transfer: creates recipient ATA, transfers tokens with sponsor as fee payer
- **[setup.ts](typescript/setup.ts)** / **[setup.rs](rust/setup.rs)** — Test setup: creates SPL mint, mints tokens, wraps into Light

## Setup

```bash
cd typescript
npm install
```

For localnet:

```bash
npm i -g @lightprotocol/zk-compression-cli@beta
light test-validator
npm run sponsor-top-ups
```

## Documentation

- [Sponsor rent top-ups](https://www.zkcompression.com/light-token/toolkits/sponsor-top-ups)
- [Light Token overview](https://www.zkcompression.com/light-token/welcome)
