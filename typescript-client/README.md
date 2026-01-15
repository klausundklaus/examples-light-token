# Light Token SDK - TypeScript Client

### Actions

- **[create-mint](actions/create-mint.ts)** - Create a light-token mint with metadata
- **[create-ata](actions/create-ata.ts)** - Create an associated light-token account
- **[load-ata](actions/load-ata.ts)** - Load cold token account to hot balance
- **[mint-to](actions/mint-to.ts)** - Mint tokens to a light-account
- **[transfer-interface](actions/transfer-interface.ts)** - Transfer between light-token, T22, and SPL accounts
- **[delegate-approve](actions/delegate-approve.ts)** - Approve delegate
- **[delegate-revoke](actions/delegate-revoke.ts)** - Revoke delegate
- **[wrap](actions/wrap.ts)** - Wrap SPL/T22 to light-token
- **[unwrap](actions/unwrap.ts)** - Unwrap light-token to SPL/T22

### Instructions

- **[create-mint](instructions/create-mint.ts)** - Build create mint instruction
- **[create-ata](instructions/create-ata.ts)** - Build create ATA instruction
- **[load-ata](instructions/load-ata.ts)** - Build load ATA instruction
- **[mint-to](instructions/mint-to.ts)** - Build mint-to instruction
- **[transfer-interface](instructions/transfer-interface.ts)** - Build transfer instruction
- **[wrap](instructions/wrap.ts)** - Wrap SPL/T22 to light-token
- **[unwrap](instructions/unwrap.ts)** - Unwrap light-token to SPL/T22

## Setup

```bash
npm install @lightprotocol/stateless.js@alpha \
            @lightprotocol/compressed-token@alpha
```

For Localnet:
```bash
npm i -g @lightprotocol/zk-compression-cli@alpha
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

Learn more [about to Light-Token here](https://www.zkcompression.com/light-token/welcome).