# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

Light Token example programs demonstrating rent-free token vaults using Light Protocol on Solana. Contains Anchor programs (escrow, fundraiser, token-swap, light-token-minter), a shared test utilities crate, TypeScript client examples, and toolkits (payments, streaming).

## Build and test

All Anchor programs live in `programs/anchor/` as a Cargo workspace. Build and test from that directory:

```bash
# Build a single program
cd programs/anchor && cargo build-sbf --manifest-path escrow/Cargo.toml

# Test a single program (runs LiteSVM-based tests, no validator needed)
cd programs/anchor && cargo test-sbf -p escrow
cd programs/anchor && cargo test-sbf -p fundraiser
cd programs/anchor && cargo test-sbf -p light-token-minter
cd programs/anchor && cargo test-sbf -p swap_example

# Run a single test by name
cd programs/anchor && cargo test-sbf -p escrow -- test_escrow_spl

# TypeScript client examples
cd typescript-client && npm install && npm run create-mint:action
```

CI runs `cargo test-sbf -p <package>` for each program (see `.github/workflows/rust-tests.yml`). Solana CLI 2.3.11, Rust 1.90.0.

## Architecture

### Programs (`programs/anchor/`)

All programs use `#[light_program]` + `#[program]` dual macros from Light Protocol. The `#[light_program]` macro generates `LightAccountVariant`, vault seed structs, and CPI signer boilerplate.

- **escrow** — Peer-to-peer token swap. `make_offer` creates an Offer account + rent-free Light vault; `take_offer` completes the swap and closes the vault. `Offer` derives `LightAccount` → generates `LightAccountVariant::Vault`, `VaultSeeds`, `OfferSeeds`.
- **fundraiser** — Crowdfunding. `initialize` creates a Fundraiser + vault; `contribute`, `check_contributions`, `refund`. `Fundraiser` uses `#[account]` only (no `LightAccount` derive) → no cold/hot vault loading possible.
- **token-swap** — AMM with `create_amm`, `create_pool`, `deposit_liquidity`, `withdraw_liquidity`, `swap_exact_tokens_for_tokens`, `create_pool_light_lp`.
- **light-token-minter** — Helper program to create Light mints with metadata and mint tokens.
- **shared-test-utils** — Reusable test helpers for SPL/T22/Light mint creation, ATA creation, SPL interface PDAs, balance verification. All programs depend on this for tests.

### Transfer routing (`shared.rs` in escrow/fundraiser)

Each program's `shared.rs` contains `transfer_tokens()` which picks the CPI path at runtime:
- Both accounts Light → `TransferCheckedCpi` (direct, no interface PDA)
- One SPL/T22 + one Light → `TransferInterfaceCpi` (cross-standard, requires SPL interface PDA)

### Token configurations tested

Each program tests 5 token standard combinations via `TokenConfig` enum:
- `Spl` — SPL mint + SPL ATAs + Light vault
- `Token2022` — T22 mint + T22 ATAs + Light vault
- `Light` — Light mint + Light ATAs + Light vault
- `LightSpl` — SPL mint, but user accounts converted to Light ATAs before escrow
- `LightT22` — T22 mint, but user accounts converted to Light ATAs before escrow

### Test setup pattern

Tests follow: `create_test_rpc()` → `setup_*_test(config)` → `create_token_account()`/`create_contributor()` → `run_*_full_flow()`. Common modules in `tests/common/mod.rs`.

### Cold/hot lifecycle (escrow only)

Light accounts auto-compress after rent expires. Escrow tests exercise the full cycle: create → `warp_to_compress` (calls `rpc.warp_epoch_forward(30)`) → `load_light_accounts` → transact. Fundraiser skips this because `Fundraiser` state lacks `LightAccount` derive.

### Local path dependencies

`light-client` and `light-program-test` use local path deps pointing to `~/Workspace/light-protocol/`. The workspace `Cargo.toml` also patches all `light-*` crates to local paths to avoid version conflicts.

## Key SDK patterns

- `CpiSigner` + `derive_light_cpi_signer!` at crate root for each program
- Constants used in `token::owner_seeds` must be `pub use`-exported from `lib.rs`
- Per-program rent sponsor PDA: `derive_rent_sponsor_pda(&program_id)` — separate from global `LIGHT_TOKEN_RENT_SPONSOR`
- `get_create_accounts_proof()` fetches validity proofs for account creation (needed for `init` instructions, not reads)
- `InitializeRentFreeConfig` sets up per-program compression config before any `#[light_account(init)]`