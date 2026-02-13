# E2E: Verify loadAta multi-account fix

## Context

The `sources.find` → `sources.filter` fix for `loadAta` is merged into `light-protocol` main. The examples project (`package.json`) links locally to `../../light-protocol/js/compressed-token` and `../../light-protocol/js/stateless.js`.

Because `tsx` resolves raw `.ts` source from `file:` links (bypassing rollup's `__BUILD_VERSION__` replacement), all test commands need `LIGHT_PROTOCOL_VERSION=V2`.

## Steps

```yaml
tasks:
  - id: 1
    name: Rebuild SDK packages with V2
    commands:
      - cd ~/Workspace/light-protocol/js/stateless.js && pnpm run build
      - cd ~/Workspace/light-protocol/js/compressed-token && LIGHT_PROTOCOL_VERSION=V2 pnpm run build
    why: Ensure dist/ reflects latest main + fix

  - id: 2
    name: Install deps in examples project
    command: cd ~/Workspace/examples-light-token/typescript-client && npm install
    depends_on: [1]

  - id: 3
    name: Run cold-only tests
    command: LIGHT_PROTOCOL_VERSION=V2 npm run test:load-ata:cold
    depends_on: [2]
    expect: All 9 scenarios pass (A1-A9)

  - id: 4
    name: Run SPL source tests
    command: LIGHT_PROTOCOL_VERSION=V2 npm run test:load-ata:spl
    depends_on: [2]
    expect: All 7 scenarios pass (B1-B7)

  - id: 5
    name: Run T22 source tests
    command: LIGHT_PROTOCOL_VERSION=V2 npm run test:load-ata:t22
    depends_on: [2]
    expect: All 5 scenarios pass (C1-C5)

  - id: 6
    name: Run load+transfer tests
    command: LIGHT_PROTOCOL_VERSION=V2 npm run test:load-ata:transfer
    depends_on: [2]
    expect: All 4 scenarios pass (D1-D4)

  - id: 7
    name: Run idempotency tests
    command: LIGHT_PROTOCOL_VERSION=V2 npm run test:load-ata:idempotency
    depends_on: [2]
    expect: All 3 scenarios pass (E1-E3)

  - id: 8
    name: Full suite (confirms sequential pass)
    command: LIGHT_PROTOCOL_VERSION=V2 npm run test:load-ata
    depends_on: [3, 4, 5, 6, 7]
    expect: All 28 scenarios pass end-to-end
```

## Prerequisites

- Local test validator running with Light programs loaded
- Solana CLI configured for localnet (`solana config set -ul`)
- Payer keypair at `~/.config/solana/id.json` with SOL balance

## Permissions needed for auto mode

| Permission | Commands | Why |
|---|---|---|
| Build SDK | `pnpm run build` in light-protocol/js/* | Rebuild dist from source |
| Install deps | `npm install` in examples project | Link local packages |
| Run tests | `npm run test:load-ata*` with env var | Execute test scripts via tsx |

All commands are local, read-only on chain (mints + transfers to localnet only), and reversible.

## If tests fail

- **V2 error persists**: tsx still hitting raw source. Try `rm -rf node_modules && npm install`
- **Indexer timeout**: Validator may need restart — `light test-validator` in separate terminal
- **Balance mismatch**: Check `coldBalance` computation in load-ata.ts line ~414
