# Payments & Wallets Toolkit

## SPL vs Light

| Operation      | SPL                                   | Light                                  |
| -------------- | ------------------------------------- | -------------------------------------- |
| Get/Create ATA | `getOrCreateAssociatedTokenAccount()` | `getOrCreateAtaInterface()`            |
| Derive ATA     | `getAssociatedTokenAddress()`         | `getAssociatedTokenAddressInterface()` |
| Transfer       | `transferChecked()`                   | `transferInterface()`                  |
| Get Balance    | `getAccount()`                        | `getAtaInterface()`                    |
| Tx History     | `getSignaturesForAddress()`           | `rpc.getSignaturesForOwnerInterface()` |
| Exit to SPL    | N/A                                   | `unwrap()`                             |

## Scripts

| File                  | Description                          | Key Function                                   |
| --------------------- | ------------------------------------ | ---------------------------------------------- |
| `send-and-receive.ts` | Send/receive payments                | `getOrCreateAtaInterface`, `transferInterface` |
| `get-balance.ts`      | Check token balance                  | `getAtaInterface`                              |
| `get-history.ts`      | Transaction history                  | `getSignaturesForOwnerInterface`               |
| `wrap.ts`             | SPL → light-token                    | `wrap`                                         |
| `unwrap.ts`           | light-token → SPL                    | `unwrap`                                       |

## Get Started

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
cp ../../.env.example .env # ...and set API_KEY
```

```bash
pnpm install

# Start local test-validator
light test-validator

# Run any script
pnpm run send-and-receive
pnpm run get-balance
pnpm run get-history
pnpm run wrap
pnpm run unwrap
```

## Documentation

Learn more [about Light-Token here](https://www.zkcompression.com/light-token/welcome).
