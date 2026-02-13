# loadAta edge case tests

28 scenarios testing `loadAta` across all combinations of cold (compressed) accounts, Light ATA state, and SPL/T22 ATAs.

## Imports: standard vs unified

The SDK exports `loadAta` from two entry points:

| Entry point | Import path | Wraps SPL/T22 ATA |
|---|---|---|
| Standard | `@lightprotocol/compressed-token` | No — cold accounts only |
| Unified | `@lightprotocol/compressed-token/unified` | Yes — wraps SPL/T22 ATA balance into Light ATA |

Tests that involve SPL or T22 wrapping (`spl-sources.ts`, `t22-sources.ts`, `load-and-transfer.ts`, `idempotency.ts`) import from the **unified** entry point. `cold-only.ts` uses the standard import since no wrapping is needed.

The unified `loadAta` sets `wrap=true` internally. For cold-only scenarios this is a no-op (no SPL/T22 ATA exists to wrap), so unified works for all cases. The standard version never wraps.

## Account limit

Each `loadAta` call decompresses up to **8 compressed accounts** per instruction. If an owner has more than 8 cold accounts, call `loadAta` repeatedly until it returns `null`.

## Run

Requires a running local validator:

```bash
light test-validator
```

Run all scenarios:

```bash
npm run test:load-ata
```

Run individually:

```bash
npm run test:load-ata:cold
npm run test:load-ata:spl
npm run test:load-ata:t22
npm run test:load-ata:transfer
npm run test:load-ata:idempotency
```

## Scenarios

Each scenario uses a fresh mint and owner keypair for isolation. Cold accounts hold 100 tokens each.

### Cold only — Light mint (`cold-only.ts`)

Tests `loadAta` with compressed accounts only (no SPL/T22 involvement).

| # | Cold | Light ATA | Expected result |
|---|------|-----------|-----------------|
| A1 | 1 | none | Creates ATA, loads 1. Balance = 100 |
| A2 | 2 | none | Creates ATA, loads 2. Balance = 200 |
| A3 | 4 | none | Creates ATA, loads 4. Balance = 400 |
| A4 | 8 | none | Creates ATA, loads 8 (max). Balance = 800 |
| A5 | 1 | exists, empty | Loads 1. Balance = 100 |
| A6 | 4 | exists, 500 | Loads 4. Balance = 900 |
| A7 | 8 | exists, 500 | Loads 8. Balance = 1300 |
| A8 | 0 | none | Returns `null`. No ATA created |
| A9 | 0 | exists, 500 | Returns `null`. Balance unchanged at 500 |

### SPL sources (`spl-sources.ts`)

Tests `loadAta` with an SPL mint. `loadAta` wraps SPL ATA balance into the Light ATA automatically.

| # | Cold | SPL ATA | Light ATA | Expected result |
|---|------|---------|-----------|-----------------|
| B1 | 0 | 1000 | none | Creates ATA, wraps SPL. Balance = 1000 |
| B2 | 1 | 1000 | none | Creates ATA, loads 1 + wraps. Balance = 1100 |
| B3 | 4 | 1000 | none | Creates ATA, loads 4 + wraps. Balance = 1400 |
| B4 | 8 | 1000 | none | Creates ATA, loads 8 + wraps. Balance = 1800 |
| B5 | 4 | 1000 | exists, 500 | Loads 4 + wraps. Balance = 1900 |
| B6 | 4 | 0 | none | Creates ATA, loads cold only. Balance = 400 |
| B7 | 0 | 0 | none | Returns `null` |

### T22 sources (`t22-sources.ts`)

Tests `loadAta` with a Token-2022 mint. Same wrapping behavior as SPL.

| # | Cold | T22 ATA | Light ATA | Expected result |
|---|------|---------|-----------|-----------------|
| C1 | 0 | 1000 | none | Creates ATA, wraps T22. Balance = 1000 |
| C2 | 1 | 1000 | none | Creates ATA, loads 1 + wraps. Balance = 1100 |
| C3 | 4 | 1000 | none | Creates ATA, loads 4 + wraps. Balance = 1400 |
| C4 | 8 | 1000 | exists, 500 | Loads 8 + wraps. Balance = 2300 |
| C5 | 4 | 0 | none | Creates ATA, loads cold only. Balance = 400 |

### Load + transfer (`load-and-transfer.ts`)

Tests composing `createLoadAtaInstructions` with `createTransferInterfaceInstruction` in a single transaction.

| # | Sources | Mint type | Expected result |
|---|---------|-----------|-----------------|
| D1 | 1 cold | Light | Load + transfer 50 in 1 tx |
| D2 | 4 cold | Light | Load + transfer 50 in 1 tx |
| D3 | 2 cold + 1000 SPL ATA | SPL | Load + wrap + transfer 50 in 1 tx |
| D4 | 2 cold + 1000 T22 ATA | T22 | Load + wrap + transfer 50 in 1 tx |

### Idempotency (`idempotency.ts`)

Tests repeated `loadAta` calls on the same ATA.

| # | Scenario | Expected result |
|---|----------|-----------------|
| E1 | Load 2 cold, then load again | Second call returns `null` |
| E2 | Load 2 cold, mint 2 more cold, load again | Second call loads the 2 new ones |
| E3 | Load 1 cold + 1000 SPL, then load again | Second call returns `null` |

## Setup (`setup.ts`)

Shared utilities used across all test files. Connects to localnet RPC and reads the payer keypair from `~/.config/solana/id.json`.

### Mint creators

| Function | Creates |
|----------|---------|
| `createLightMint()` | Light mint (no token program) |
| `createSplMint()` | SPL mint + Light interface PDA |
| `createT22Mint()` | T22 mint + Light interface PDA |

### Account creators

| Function | Does |
|----------|------|
| `createMultipleCompressed(mint, owner, count, amountEach)` | Calls `mintToCompressed` N times to create N cold accounts |
| `createLightAtaWithBalance(mint, owner, amount?)` | Creates Light ATA, optionally mints tokens to it |
| `getSplAtaWithBalance(mint, owner, amount)` | Creates SPL ATA via `@solana/spl-token`, mints tokens to it |
| `getT22AtaWithBalance(mint, owner, amount)` | Creates T22 ATA via `@solana/spl-token`, mints tokens to it |

### Balance readers

| Function | Returns |
|----------|---------|
| `getCompressedCount(owner, mint)` | Number of compressed accounts for owner + mint |
| `getLightAtaBalance(ata, owner, mint)` | Light ATA token balance (0 if account doesn't exist) |
| `getSplAtaBalance(ata, programId?)` | SPL/T22 ATA balance (0 if account doesn't exist) |

### Assertions

| Function | Does |
|----------|------|
| `assert(condition, msg)` | Throws on failure |
| `logScenario(name, expected, actual)` | Prints `[PASS]` or `[FAIL]` with values, throws on mismatch |
