//! Light Token AMM: automated market maker with rent-free pool vaults.
//!
//! This test shows constant-product AMM patterns adapted for Light Token.
//!
//! 1. **Authority PDA owns the pool vaults** — not the pool account. This lets
//!    the authority sign vault operations (deposit, swap, withdraw) without
//!    needing account data (see `create_pool.rs`
//!    `#[light_account(init, token::owner = pool_authority)]`).
//!
//! 2. **Pool vaults are rent-free** — they're Light Token accounts sponsored by
//!    `RENT_SPONSOR`, eliminating ~0.002 SOL rent per pool vault.
//!
//! 3. **Validity proof for pool creation** — `get_create_accounts_proof()`
//!    fetches a validity proof that the vault PDAs' derived addresses do not
//!    yet exist in Light's address tree. Only needed in `create_pool` /
//!    `create_pool_light_lp` (creating new state); deposit, swap, and withdraw
//!    read existing accounts and need no proof.
//!
//! ## Token scenarios
//!
//! The pool vaults are always Light Token accounts. The user's account type
//! determines which CPI path `transfer_tokens()` in `shared.rs` selects:
//!
//! | Test | Mint | User accounts | LP mint | Transfer path | Purpose |
//! |------|------|---------------|---------|---------------|---------|
//! | `test_swap_spl` | SPL | SPL ATAs | SPL | `TransferInterfaceCpi` | SPL mints work with Light vault |
//! | `test_swap_t22` | Token 2022 | Token 2022 ATAs | Token 2022 | `TransferInterfaceCpi` | Token 2022 mints work with Light Token vault |
//! | `test_swap_light` | Light | Light ATAs | SPL | `TransferCheckedCpi` | Light Token mints, SPL LP mint |
//! | `test_swap_spl_light` | SPL | Light ATAs | SPL | `TransferCheckedCpi` | SPL mint, converted user accounts |
//! | `test_swap_t22_light` | Token 2022 | Light ATAs | Token 2022 | `TransferCheckedCpi` | Token 2022 mint, converted user accounts |
//! | `test_swap_full_light` | Light | Light ATAs | Light | `TransferCheckedCpi` | Fully rent-free (Light LP mint) |
//!
//! For `Spl`/`Token2022`: user accounts are SPL/Token 2022, vaults are Light →
//! `TransferInterfaceCpi` (with SPL interface PDA).
//!
//! For `Light`/`LightToLight`/`LightSpl`/`LightT22`: all accounts are Light Token →
//! `TransferCheckedCpi` (no interface PDA needed).
//!
//! `LightSpl`/`LightT22` convert tokens from SPL/Token 2022 associated token accounts
//! into Light Token accounts *before* the AMM interactions start (in
//! `create_trader` / setup). The AMM itself only sees Light Token accounts.
//!
//! ## Pool creation modes
//!
//! `LightToLight` uses `create_pool_light_lp` which creates pool vaults via
//! explicit `CreateTokenAccountCpi` and the LP mint as a Light Token mint.
//! All other configs use `create_pool` with macro-initialized vaults and
//! an SPL/Token 2022 LP mint.
//!
//! ## Cold/hot lifecycle
//!
//! `Pool` and `Amm` use standard `#[account]` (no `LightAccount` derive), so
//! `#[light_program]` does not generate `VaultSeeds` / `LightAccountVariant`
//! needed by `create_load_instructions`. The test does not exercise cold/hot
//! lifecycle. See the escrow tests for full cold/hot coverage.

mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::token;
use common::{
    create_test_rpc, create_trader, get_token_balance, setup_amm_test, AmmTestContext, TokenConfig,
};
use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
use light_token::spl_interface::find_spl_interface_pda;
use shared_test_utils::{
    helpers::verify_light_token_balance,
    light_tokens::create_light_ata,
    spl_tokens::create_spl_ata,
    t22_tokens::create_t22_ata,
    Indexer, Rpc, TestRpc, COMPRESSIBLE_CONFIG_V1, CPI_AUTHORITY_PDA, LIGHT_TOKEN_PROGRAM_ID,
    RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ============================================================================
// Tests — one per token configuration, plus a standalone AMM creation test.
// ============================================================================

/// SPL mint with SPL ATAs, SPL LP mint, and Light Token pool vaults.
///
/// All transfers use `TransferInterfaceCpi` (SPL ↔ Light Token vault)
/// with SPL interface PDAs. This is the baseline: same mint type as standard
/// SPL AMM, but pool vaults are rent-free Light Token accounts.
#[tokio::test]
async fn test_swap_spl() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Spl).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// Token 2022 mint with Token 2022 ATAs, Token 2022 LP mint, and Light Token pool vaults.
///
/// All transfers use `TransferInterfaceCpi` (Token 2022 ↔ Light Token vault)
/// with SPL interface PDAs.
#[tokio::test]
async fn test_swap_t22() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Token2022).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// Light Token mint + Light Token user accounts, SPL LP mint, and Light Token pool vaults.
///
/// All token A/B transfers are Light-to-Light (`TransferCheckedCpi`).
/// LP mint is SPL because `create_pool` uses Anchor's `init` macro
/// which only supports SPL/T22 mints.
#[tokio::test]
async fn test_swap_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Light).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// SPL mint + Light Token user accounts, SPL LP mint. SPL tokens converted to
/// Light Token accounts in setup via `transfer_spl_to_light`.
///
/// All transfers use `TransferCheckedCpi`.
#[tokio::test]
async fn test_swap_spl_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::LightSpl).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// Token 2022 mint + Light Token user accounts, Token 2022 LP mint. Token 2022 tokens
/// converted to Light Token accounts in setup via `transfer_spl_to_light`.
///
/// All transfers use `TransferCheckedCpi`.
#[tokio::test]
async fn test_swap_t22_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::LightT22).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// Light Token mints + Light Token user accounts + Light Token LP mint.
///
/// Uses `create_pool_light_lp` to create the LP mint as a Light Token mint via CPI.
/// All transfers use `TransferCheckedCpi`.
#[tokio::test]
async fn test_swap_full_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::LightToLight).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// AMM creation is config-independent (fee + admin only). Verifies that
/// `create_amm` works in isolation without pool creation.
#[tokio::test]
async fn test_create_amm() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Spl).await;
    create_amm(&mut rpc, &ctx, 0).await;
}

// ============================================================================
// Full Flow
// ============================================================================

/// Run the full AMM flow for any token configuration: SPL, Token 2022, Light.
///
/// 1. Create AMM with 2.5% fee
/// 2. Create pool with Light Token vaults (standard or LightToLight path)
/// 3. Deposit initial liquidity, receive LP tokens
/// 4. Create trader with funded accounts
/// 5. Swap A→B
/// 6. Swap B→A (half of received B)
/// 7. Withdraw half of LP tokens
/// 8. Verify final LP balance
async fn run_amm_full_flow<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
) {
    let token_program = ctx.token_config.token_program_id();
    let liquidity_token_program = match ctx.token_config {
        TokenConfig::Light | TokenConfig::LightSpl | TokenConfig::Spl => token::ID,
        TokenConfig::Token2022 | TokenConfig::LightT22 => spl_token_2022::ID,
        TokenConfig::LightToLight => Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
    };

    create_amm(rpc, ctx, 250).await;

    create_pool(rpc, ctx, token_program, liquidity_token_program).await;

    verify_light_token_balance(rpc, ctx.pool_account_a, 0, "pool_account_a (initial)").await;
    verify_light_token_balance(rpc, ctx.pool_account_b, 0, "pool_account_b (initial)").await;

    let deposit_amount = 1_000_000_000u64; // 1 token
    let depositor_liquidity_ata = deposit_liquidity(
        rpc,
        ctx,
        token_program,
        liquidity_token_program,
        deposit_amount,
    )
    .await;

    verify_light_token_balance(
        rpc,
        ctx.pool_account_a,
        deposit_amount,
        "pool_account_a (after deposit)",
    )
    .await;
    verify_light_token_balance(
        rpc,
        ctx.pool_account_b,
        deposit_amount,
        "pool_account_b (after deposit)",
    )
    .await;

    let trader_initial_a = 100_000_000u64; // 0.1 tokens
    let (trader, trader_ata_a, trader_ata_b) = create_trader(rpc, ctx, trader_initial_a).await;

    let swap_input = 10_000_000u64; // 0.01 tokens
    swap_exact_tokens_for_tokens(
        rpc,
        ctx,
        &trader,
        trader_ata_a,
        trader_ata_b,
        token_program,
        true,
        swap_input,
        1,
    )
    .await;

    let trader_b_balance = get_token_balance(rpc, trader_ata_b).await;
    assert!(trader_b_balance > 0, "Trader should have received token B");

    let swap_b_input = trader_b_balance / 2;
    swap_exact_tokens_for_tokens(
        rpc,
        ctx,
        &trader,
        trader_ata_a,
        trader_ata_b,
        token_program,
        false,
        swap_b_input,
        1,
    )
    .await;

    let lp_balance = get_token_balance(rpc, depositor_liquidity_ata).await;
    let withdraw_amount = lp_balance / 2;
    withdraw_liquidity(
        rpc,
        ctx,
        token_program,
        liquidity_token_program,
        depositor_liquidity_ata,
        withdraw_amount,
    )
    .await;

    let lp_balance_after = get_token_balance(rpc, depositor_liquidity_ata).await;
    assert_eq!(
        lp_balance_after,
        lp_balance - withdraw_amount,
        "LP tokens should be burned"
    );
}

// ============================================================================
// Instruction Helpers
// ============================================================================

/// Send `create_amm` instruction with the given fee (basis points, e.g. 250 = 2.5%).
async fn create_amm<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    fee: u16,
) {
    let accounts = swap_example::accounts::CreateAmm {
        amm: ctx.amm_pda,
        admin: ctx.payer.pubkey(),
        payer: ctx.payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };

    let data = swap_example::instruction::CreateAmm {
        id: ctx.amm_id,
        fee,
    };

    let ix = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    rpc.create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("create_amm should succeed");
}

/// Fetch validity proof and send `create_pool` or `create_pool_light_lp` instruction.
///
/// LightToLight proves LP mint signer (vaults created via explicit CPI).
/// Standard proves pool vault PDAs (vaults created via macro).
async fn create_pool<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    token_program: Pubkey,
    liquidity_token_program: Pubkey,
) {
    // Validity proof: verifies the vault PDAs (or LP mint signer for LightToLight)
    // do not yet exist in the address tree. Required for Light Token account creation.
    let proof_inputs = if ctx.token_config.uses_light_lp_mint() {
        vec![CreateAccountsProofInput::mint(
            ctx.lp_mint_signer
                .expect("LightToLight config should have lp_mint_signer"),
        )]
    } else {
        vec![
            CreateAccountsProofInput::pda(ctx.pool_account_a),
            CreateAccountsProofInput::pda(ctx.pool_account_b),
        ]
    };

    let proof_result = get_create_accounts_proof(rpc, &ctx.program_id, proof_inputs)
        .await
        .unwrap();

    if ctx.token_config.uses_light_lp_mint() {
        let lp_mint_signer = ctx
            .lp_mint_signer
            .expect("LightToLight config should have lp_mint_signer");

        let accounts = swap_example::accounts::CreatePoolLightLp {
            amm: ctx.amm_pda,
            pool: ctx.pool_pda,
            pool_authority: ctx.pool_authority,
            lp_mint_signer,
            mint_liquidity: ctx.mint_liquidity,
            mint_a: ctx.mint_a_pubkey,
            mint_b: ctx.mint_b_pubkey,
            pool_vault_a: ctx.pool_account_a,
            pool_vault_b: ctx.pool_account_b,
            fee_payer: ctx.payer.pubkey(),
            token_program,
            compression_config: ctx.compression_config,
            light_token_config: COMPRESSIBLE_CONFIG_V1,
            light_token_rent_sponsor: RENT_SPONSOR,
            light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            light_token_cpi_authority: CPI_AUTHORITY_PDA,
            system_program: solana_sdk::system_program::ID,
        };

        let data = swap_example::instruction::CreatePoolLightLp {
            params: swap_example::CreatePoolLightLpParams {
                create_accounts_proof: proof_result.create_accounts_proof,
                pool_account_a_bump: ctx.pool_a_bump,
                pool_account_b_bump: ctx.pool_b_bump,
                lp_mint_signer_bump: ctx.lp_mint_signer_bump,
                pool_authority_bump: ctx.pool_authority_bump,
            },
        };

        let ix = Instruction {
            program_id: ctx.program_id,
            accounts: [
                accounts.to_account_metas(None),
                proof_result.remaining_accounts,
            ]
            .concat(),
            data: data.data(),
        };

        rpc.create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer])
            .await
            .expect("create_pool_light_lp should succeed");
    } else {
        let accounts = swap_example::accounts::CreatePool {
            amm: ctx.amm_pda,
            pool: ctx.pool_pda,
            pool_authority: ctx.pool_authority,
            mint_liquidity: ctx.mint_liquidity,
            mint_a: ctx.mint_a_pubkey,
            mint_b: ctx.mint_b_pubkey,
            pool_account_a: ctx.pool_account_a,
            pool_account_b: ctx.pool_account_b,
            fee_payer: ctx.payer.pubkey(),
            token_program,
            liquidity_token_program,
            light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            system_program: solana_sdk::system_program::ID,
            light_token_config: COMPRESSIBLE_CONFIG_V1,
            light_token_rent_sponsor: RENT_SPONSOR,
            light_token_cpi_authority: CPI_AUTHORITY_PDA,
        };

        let data = swap_example::instruction::CreatePool {
            params: swap_example::CreatePoolParams {
                create_accounts_proof: proof_result.create_accounts_proof,
                pool_account_a_bump: ctx.pool_a_bump,
                pool_account_b_bump: ctx.pool_b_bump,
            },
        };

        let ix = Instruction {
            program_id: ctx.program_id,
            accounts: [
                accounts.to_account_metas(None),
                proof_result.remaining_accounts,
            ]
            .concat(),
            data: data.data(),
        };

        rpc.create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer])
            .await
            .expect("create_pool should succeed");
    }

    // pool_authority needs lamports to pay rent top-ups when
    // transferring from pool vaults to Light Token user accounts.
    if ctx.token_config.uses_light_user_accounts() {
        rpc.airdrop_lamports(&ctx.pool_authority, 1_000_000_000)
            .await
            .expect("Fund pool_authority for rent top-ups");
    }
}

/// Create depositor LP ATA and send `deposit_liquidity` instruction.
///
/// Returns the depositor's LP token account address.
async fn deposit_liquidity<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    token_program: Pubkey,
    liquidity_token_program: Pubkey,
    amount: u64,
) -> Pubkey {
    let depositor_liquidity_ata = match ctx.token_config {
        TokenConfig::Spl | TokenConfig::LightSpl | TokenConfig::Light => {
            create_spl_ata(
                rpc,
                &ctx.payer,
                &ctx.mint_liquidity,
                &ctx.depositor.pubkey(),
            )
            .await
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            create_t22_ata(
                rpc,
                &ctx.payer,
                &ctx.mint_liquidity,
                &ctx.depositor.pubkey(),
            )
            .await
        }
        TokenConfig::LightToLight => {
            create_light_ata(
                rpc,
                &ctx.payer,
                &ctx.mint_liquidity,
                &ctx.depositor.pubkey(),
            )
            .await
        }
    };

    let accounts = swap_example::accounts::DepositLiquidity {
        pool: ctx.pool_pda,
        pool_authority: ctx.pool_authority,
        depositor: ctx.depositor.pubkey(),
        mint_liquidity: ctx.mint_liquidity,
        mint_a: ctx.mint_a_pubkey,
        mint_b: ctx.mint_b_pubkey,
        pool_account_a: ctx.pool_account_a,
        pool_account_b: ctx.pool_account_b,
        depositor_account_liquidity: depositor_liquidity_ata,
        depositor_account_a: ctx.depositor_ata_a,
        depositor_account_b: ctx.depositor_ata_b,
        payer: ctx.payer.pubkey(),
        token_program,
        liquidity_token_program,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a: if ctx.token_config.uses_light_user_accounts() { None } else { Some(ctx.spl_interface_pda_a) },
        spl_interface_pda_b: if ctx.token_config.uses_light_user_accounts() { None } else { Some(ctx.spl_interface_pda_b) },
    };

    let (_, spl_interface_bump_a) = find_spl_interface_pda(&ctx.mint_a_pubkey, false);
    let (_, spl_interface_bump_b) = find_spl_interface_pda(&ctx.mint_b_pubkey, false);

    let data = swap_example::instruction::DepositLiquidity {
        amount_a: amount,
        amount_b: amount,
        spl_interface_bump_a,
        spl_interface_bump_b,
    };

    let ix = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    rpc.create_and_send_transaction(
        &[ix],
        &ctx.payer.pubkey(),
        &[&ctx.payer, &ctx.depositor],
    )
    .await
    .expect("deposit_liquidity should succeed");

    depositor_liquidity_ata
}

/// Send `swap_exact_tokens_for_tokens` instruction.
///
/// `swap_a = true` swaps A→B; `swap_a = false` swaps B→A.
async fn swap_exact_tokens_for_tokens<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    trader: &Keypair,
    trader_ata_a: Pubkey,
    trader_ata_b: Pubkey,
    token_program: Pubkey,
    swap_a: bool,
    input_amount: u64,
    min_output_amount: u64,
) {
    let accounts = swap_example::accounts::SwapExactTokensForTokens {
        amm: ctx.amm_pda,
        pool: ctx.pool_pda,
        pool_authority: ctx.pool_authority,
        trader: trader.pubkey(),
        mint_a: ctx.mint_a_pubkey,
        mint_b: ctx.mint_b_pubkey,
        pool_account_a: ctx.pool_account_a,
        pool_account_b: ctx.pool_account_b,
        trader_account_a: trader_ata_a,
        trader_account_b: trader_ata_b,
        payer: ctx.payer.pubkey(),
        token_program,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a: if ctx.token_config.uses_light_user_accounts() { None } else { Some(ctx.spl_interface_pda_a) },
        spl_interface_pda_b: if ctx.token_config.uses_light_user_accounts() { None } else { Some(ctx.spl_interface_pda_b) },
    };

    let (_, spl_interface_bump_a) = find_spl_interface_pda(&ctx.mint_a_pubkey, false);
    let (_, spl_interface_bump_b) = find_spl_interface_pda(&ctx.mint_b_pubkey, false);

    let data = swap_example::instruction::SwapExactTokensForTokens {
        swap_a,
        input_amount,
        min_output_amount,
        spl_interface_bump_a,
        spl_interface_bump_b,
    };

    let ix = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    let direction = if swap_a { "A->B" } else { "B->A" };
    rpc.create_and_send_transaction(&[ix], &ctx.payer.pubkey(), &[&ctx.payer, trader])
        .await
        .unwrap_or_else(|e| panic!("swap {direction} should succeed: {e}"));
}

/// Send `withdraw_liquidity` instruction to burn LP tokens and receive pool tokens.
async fn withdraw_liquidity<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    token_program: Pubkey,
    liquidity_token_program: Pubkey,
    depositor_liquidity_ata: Pubkey,
    amount: u64,
) {
    let accounts = swap_example::accounts::WithdrawLiquidity {
        amm: ctx.amm_pda,
        pool: ctx.pool_pda,
        pool_authority: ctx.pool_authority,
        depositor: ctx.depositor.pubkey(),
        mint_liquidity: ctx.mint_liquidity,
        mint_a: ctx.mint_a_pubkey,
        mint_b: ctx.mint_b_pubkey,
        pool_account_a: ctx.pool_account_a,
        pool_account_b: ctx.pool_account_b,
        depositor_account_liquidity: depositor_liquidity_ata,
        depositor_account_a: ctx.depositor_ata_a,
        depositor_account_b: ctx.depositor_ata_b,
        payer: ctx.payer.pubkey(),
        token_program,
        liquidity_token_program,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a: if ctx.token_config.uses_light_user_accounts() { None } else { Some(ctx.spl_interface_pda_a) },
        spl_interface_pda_b: if ctx.token_config.uses_light_user_accounts() { None } else { Some(ctx.spl_interface_pda_b) },
    };

    let (_, spl_interface_bump_a) = find_spl_interface_pda(&ctx.mint_a_pubkey, false);
    let (_, spl_interface_bump_b) = find_spl_interface_pda(&ctx.mint_b_pubkey, false);

    let data = swap_example::instruction::WithdrawLiquidity {
        amount,
        spl_interface_bump_a,
        spl_interface_bump_b,
    };

    let ix = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    rpc.create_and_send_transaction(
        &[ix],
        &ctx.payer.pubkey(),
        &[&ctx.payer, &ctx.depositor],
    )
    .await
    .expect("withdraw_liquidity should succeed");
}
