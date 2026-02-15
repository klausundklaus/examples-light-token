# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Light Token examples repository demonstrating rent-free token operations on Solana. Light Token reduces mint and token account costs by 200x through sponsored rent-exemption.

## Build and test commands

### TypeScript client

```bash
# Install dependencies (from repo root — npm workspaces)
npm install

# Run from typescript-client directory
cd typescript-client
npm run test:actions              # Run all action examples (chained sequentially)
npm run test:instructions         # Run all instruction examples (chained sequentially)

# Individual examples (pattern: <operation>:<level>)
npm run create-mint:action        # Create light-token mint
npm run create-mint:instruction   # Create mint with manual instruction building
npm run transfer-interface:action # Transfer between account types
npm run wrap:action               # Wrap SPL/T22 to light-token
npm run unwrap:action             # Unwrap light-token to SPL/T22
npm run load-ata:action           # Load compressed tokens into light ATA
npm run delegate:approve          # Approve delegate
npm run delegate:revoke           # Revoke delegate

# All examples require a running validator — see "Local development" below
```

### Rust client

```bash
cd rust-client
cargo run --example action_create_mint
cargo run --example action_transfer_interface
cargo run --example instruction_transfer_interface
```

### Anchor programs

```bash
cd programs/anchor

anchor build

# Test single program (recommended — must use single thread)
cargo test-sbf -p <package-name> -- --test-threads=1

# Package names: light-token-anchor-transfer-interface, light-token-anchor-mint-to,
# counter, create-and-transfer
```

### Privy Node.js (devnet)

```bash
cd privy/nodejs
npm install
cp .env.example .env  # fill in Privy credentials + Helius RPC URL

# App operations (Privy-signed server wallet)
npm run transfer      # Light-token ATA → ATA transfer (auto-loads cold balance)
npm run wrap          # SPL/T22 → light-token ATA
npm run unwrap        # Light-token ATA → SPL/T22
npm run load          # Consolidate cold + SPL + T22 into light-token ATA
npm run balances      # Query balance breakdown (hot, cold, SPL/T22, SOL)
npm run history       # Transaction history for light-token interface ops

# Setup helpers live in privy/scripts/ (separate workspace, uses local keypair)
cd ../scripts
npm run mint:spl-and-wrap <recipient>    # Create mint + interface PDA + fund treasury
npm run mint:spl <mint> <recipient>     # Mint SPL/T22 tokens to existing mint
npm run register:spl-interface <mint>   # Register interface PDA on existing mint
```

### Privy React (devnet — WIP)

```bash
cd privy/react
npm install
cp .env.example .env  # fill in VITE_PRIVY_APP_ID and VITE_HELIUS_RPC_URL

npm run dev      # Vite dev server
npm run build    # Production build
```

### Local development

```bash
# Start test validator with Light programs (requires light CLI)
light test-validator

# Validator health check
curl http://127.0.0.1:8784/health
```

## Architecture

### Workspace layout

Root `package.json` defines npm workspaces: `typescript-client`, `toolkits/payments-and-wallets`, `privy/nodejs`, `privy/react`, and `privy/scripts`. Dependencies are hoisted to root.

- `typescript-client/` — Core examples: `actions/` (high-level) and `instructions/` (low-level)
- `rust-client/` — Rust client examples as cargo examples
- `programs/anchor/` — On-chain Anchor programs
  - `basic-macros/` — Declarative `#[light_account]` macro pattern
  - `basic-instructions/` — Explicit CPI calls to light-token program
  - `create-and-transfer/` — Combined macro + CPI example
- `toolkits/` — Domain-specific implementations
  - `payments-and-wallets/` — Wallet integration patterns (uses `@lightprotocol/compressed-token/unified` subpath)
  - `streaming-tokens/` — Laserstream-based token indexing
- `privy/` — Privy wallet integration examples (devnet)
  - `nodejs/` — Server-side scripts using `@privy-io/node` with server wallet signing
  - `react/` — Browser app using `@privy-io/react-auth` with embedded wallet signing (WIP)

### TypeScript: actions vs instructions

Every example exists at two abstraction levels:

**Actions** (`typescript-client/actions/`): Call high-level SDK functions that build, sign, and send transactions in one call. Functions like `createMintInterface`, `transferInterface`, `wrap` return a transaction signature directly.

**Instructions** (`typescript-client/instructions/`): Build `TransactionInstruction` objects manually, assemble into `Transaction`, and call `sendAndConfirmTransaction`. Requires fetching tree info and validity proofs yourself:
- `getBatchAddressTreeInfo()` — address tree for new account creation
- `selectStateTreeInfo(await rpc.getStateTreeInfos())` — state tree selection
- `rpc.getValidityProofV2([], [{address, treeInfo}])` — ZK validity proof
- `ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 })` — required CU budget

### Boilerplate pattern

Every TypeScript example follows the same setup:

```typescript
import "dotenv/config";
import { createRpc } from "@lightprotocol/stateless.js";

// localnet (default — no args):
const rpc = createRpc();
// devnet:
// const rpc = createRpc(`https://devnet.helius-rpc.com?api-key=${process.env.API_KEY!}`);

// Payer from local Solana keypair:
const payer = Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8")))
);
```

### Token flow concepts

- **Light-token ATA**: Associated token account derived via `getAssociatedTokenAddressInterface(mint, owner)`. Created with `createAtaInterface` or `createAtaInterfaceIdempotent`.
- **Wrap**: SPL/T22 tokens → light-token ATA. Requires SPL interface PDA registered via `createSplInterface`.
- **Unwrap**: Light-token ATA → SPL/T22 tokens.
- **Load ATA**: Compressed tokens (cold storage) → light-token ATA (hot balance). Uses `loadAta` action or `createLoadAtaInstructions` + `buildAndSignTx`/`sendAndConfirmTx`.
- **Decompress**: Light-token ATA → SPL T22 account. Used as setup step before wrapping.

### Two on-chain patterns (Anchor programs)

**Macro pattern** (`basic-macros/`): Declarative `#[light_account(init)]` attribute and `LightAccounts` derive macro.

**CPI pattern** (`basic-instructions/`): Explicit CPI calls like `TransferInterfaceCpi`.

### Privy Node.js architecture (`privy/nodejs/`)

Server-side scripts that sign transactions via Privy's wallet API instead of a local keypair. Each script in `src/` is standalone and runnable with `tsx`. All scripts target **devnet**.

**Script pattern**: Each file exports a reusable async function AND has a `// --- main ---` block at the bottom that imports config values and runs it. Scripts are both importable as modules and directly runnable via `npm run <script>`.

**Signing**: Each script inlines its own Privy signing — no shared utilities between scripts. Pattern: build `Transaction` → serialize with `requireAllSignatures: false` → `privy.wallets().solana().signTransaction(walletId, { transaction, authorization_context })` → `sendRawTransaction` → `confirmTransaction`. Scripts that receive `TransactionInstruction[][]` from the SDK loop over batches sequentially.

**Two workspaces, two signing modes**:

- **App operations** (`privy/nodejs/src/*.ts`): Privy server wallet signing. Six scripts: `transfer`, `wrap`, `unwrap`, `load`, `balances`, `get-transaction-history`.
- **Setup helpers** (`privy/scripts/src/*.ts`): Separate npm workspace. Uses local filesystem keypair (`~/.config/solana/id.json`), not Privy. Scripts: `mint-spl-and-wrap`, `mint-spl`, `register-spl-interface`.

**Config**: All env vars centralized in `src/config.ts` with validation. Exports `TREASURY_WALLET_ID`, `TREASURY_WALLET_ADDRESS`, `TREASURY_AUTHORIZATION_KEY`, `HELIUS_RPC_URL`, `TEST_MINT`, and convenience defaults (`DEFAULT_TEST_RECIPIENT`, `DEFAULT_AMOUNT`, `DEFAULT_DECIMALS`). Scripts import from `./config.js` (ESM `.js` extension required).

**Per-script details**:

- **transfer** — Calls `createTransferInterfaceInstructions` (from `/unified` subpath) which returns `TransactionInstruction[][]`. Auto-loads cold balance before transferring. Usually one tx.
- **wrap** — Manually assembles 3 instructions: `setComputeUnitLimit(200_000)` + `createAssociatedTokenAccountInterfaceIdempotentInstruction` + `createWrapInstruction`. Calls `getSplInterfaceInfos` (from root `@lightprotocol/compressed-token`, not `/unified`) to get `tokenProgram` and derive the correct SPL ATA.
- **unwrap** — Calls `createUnwrapInstructions` from `/unified` subpath. Returns batched instructions handling load + unwrap together.
- **load** — Calls `createLoadAtaInstructions`. Can return empty array (nothing to load). Derives light-token ATA via `getAssociatedTokenAddressInterface`.
- **balances** — Queries 4 sources: SOL (`getBalance`), hot (`getAtaInterface`), cold (`getCompressedTokenBalancesByOwnerV2`), SPL + T22 (raw `getTokenAccountsByOwner` with manual buffer parsing at offset 64). Hardcodes decimals=9.
- **get-transaction-history** — Calls `getSignaturesForOwnerInterface`, a light-token specific RPC method.

**Import subpaths**: Most light-token instructions come from `@lightprotocol/compressed-token/unified`. Exception: `getSplInterfaceInfos` comes from root `@lightprotocol/compressed-token`.

### Key dependencies

- `@lightprotocol/compressed-token` beta — TypeScript token client (also exports `/unified` subpath for unwrap, load, balances)
- `@lightprotocol/stateless.js` beta — TypeScript RPC client (`createRpc`, `buildAndSignTx`, `sendAndConfirmTx`)
- `@solana/web3.js` 1.98.x — Solana web3 (v1, not v2)
- `@solana/spl-token` 0.4.x — SPL token operations (used for T22 interop)
- `@privy-io/node` ^0.1.0-alpha.2 — Server-side Privy SDK (Node.js scripts)
- `light-sdk` 0.19.x — Rust core SDK
- `light-token` 0.4.x — Rust token operations
- Anchor 0.31.1, Solana SDK 2.2, Rust 1.90.0

## Environment setup

Copy `.env.example` to `.env` and set `API_KEY` for devnet/mainnet RPC access. Localnet uses default endpoints (no env needed).

## Documentation

https://www.zkcompression.com/light-token/welcome
