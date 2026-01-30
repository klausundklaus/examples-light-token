# Light Token - TypeScript Client

TypeScript client examples for light-token-sdk.

- **create-mint** - Create a light-token mint with metadata
  - [Action](actions/create-mint.ts) | [Instruction](instructions/create-mint.ts)
- **create-ata** - Create an associated light-token account
  - [Action](actions/create-ata.ts) | [Instruction](instructions/create-ata.ts)
- **load-ata** - Load token accounts from light-token, compressed tokens, SPL/T22 to one unified balance
  - [Action](actions/load-ata.ts) | [Instruction](instructions/load-ata.ts)
- **mint-to** - Mint tokens to a light-account
  - [Action](actions/mint-to.ts) | [Instruction](instructions/mint-to.ts)
- **transfer-interface** - Transfer between light-token, T22, and SPL accounts
  - [Action](actions/transfer-interface.ts) | [Instruction](instructions/transfer-interface.ts)
- **wrap** - Wrap SPL/T22 to light-token
  - [Action](actions/wrap.ts) | [Instruction](instructions/wrap.ts)
- **unwrap** - Unwrap light-token to SPL/T22
  - [Action](actions/unwrap.ts) | [Instruction](instructions/unwrap.ts)
- **delegate-approve** - Approve delegate
  - [Action](actions/delegate-approve.ts)
- **delegate-revoke** - Revoke delegate
  - [Action](actions/delegate-revoke.ts)

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

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
