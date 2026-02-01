# Light Token - TypeScript Client

TypeScript client examples for light-token-sdk.

- **[create-mint](actions/create-mint.ts)** - Create a light-token mint with metadata
- **[create-spl-mint](actions/create-spl-mint.ts)** - Create an SPL mint with SPL interface PDA
- **[create-t22-mint](actions/create-t22-mint.ts)** - Create a Token-2022 mint with SPL interface PDA
- **[create-spl-interface](actions/create-spl-interface.ts)** - Create SPL interface PDA for an existing mint
- **[create-ata](actions/create-ata.ts)** - Create an associated light-token account
- **[load-ata](actions/load-ata.ts)** - Load token accounts from light-token, compressed tokens, SPL/T22 to one unified balance
- **[mint-to](actions/mint-to.ts)** - Mint tokens to a light-account
- **[transfer-interface](actions/transfer-interface.ts)** - Transfer between light-token, T22, and SPL accounts
- **[delegate-approve](actions/delegate-approve.ts)** - Approve delegate
- **[delegate-revoke](actions/delegate-revoke.ts)** - Revoke delegate
- **[wrap](actions/wrap.ts)** - Wrap SPL/T22 to light-token
- **[unwrap](actions/unwrap.ts)** - Unwrap light-token to SPL/T22

### Instructions

- **[create-mint](instructions/create-mint.ts)** - Build create mint instruction
- **[create-spl-mint](instructions/create-spl-mint.ts)** - Build SPL mint + SPL interface PDA instructions
- **[create-t22-mint](instructions/create-t22-mint.ts)** - Build Token-2022 mint + SPL interface PDA instructions
- **[create-spl-interface](instructions/create-spl-interface.ts)** - Build SPL interface PDA instruction
- **[create-ata](instructions/create-ata.ts)** - Build create ATA instruction
- **[load-ata](instructions/load-ata.ts)** - Build load ATA instruction
- **[mint-to](instructions/mint-to.ts)** - Build mint-to instruction
- **[transfer-interface](instructions/transfer-interface.ts)** - Build transfer instruction
- **[wrap](instructions/wrap.ts)** - Wrap SPL/T22 to light-token
- **[unwrap](instructions/unwrap.ts)** - Unwrap light-token to SPL/T22

## Setup

```bash
npm install @lightprotocol/stateless.js@beta \
            @lightprotocol/compressed-token@beta
```

For Localnet:
```bash
npm i -g @lightprotocol/zk-compression-cli@beta
```

For Devnet:
```bash
cp ../.env.example .env # ...and set API_KEY
```

## Run

```bash
npm run create-mint:action
npm run mint-to:action
npm run transfer-interface:action
# See package.json for all scripts
```
```bash
npm run create-mint:instruction
npm run create-ata:instruction
npm run load-ata:instruction
# See package.json for all scripts
```

## Documentation

Learn more [about Light Token here](https://www.zkcompression.com/light-token/welcome).
