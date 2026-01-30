//! Common test utilities for token-swap AMM tests.
//!
//! This module provides shared test helpers for different token type combinations:
//! - SPL mint + SPL ATAs
//! - T22 mint + T22 ATAs
//! - SPL mint + Light user accounts
//! - T22 mint + Light user accounts
//!
//! Light mint + Light ATAs is not yet supported (requires Light mint creation).

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token;
use shared_test_utils::{
    helpers::verify_light_token_balance,
    light_tokens::{create_light_ata, create_light_mint, mint_light_tokens},
    setup::initialize_rent_free_config,
    spl_interface::{create_spl_interface_pda, transfer_spl_to_light},
    spl_tokens::{create_spl_ata, create_spl_mint, mint_spl_tokens},
    t22_tokens::{create_t22_ata, create_t22_mint, mint_t22_tokens},
    CreateAccountsProofResult, Indexer, LightProgramTest, MintType, ProgramTestConfig, Rpc,
    TestRpc, CPI_AUTHORITY_PDA, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_MINTER_PROGRAM_ID,
    LIGHT_TOKEN_PROGRAM_ID, LIGHT_TOKEN_RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022::pod::PodAccount;

/// Token configuration for parameterized tests
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum TokenConfig {
    /// SPL mint + SPL ATAs
    Spl,
    /// T22 mint + T22 ATAs
    Token2022,
    /// SPL mint + Light user accounts (offchain SPL->Light conversion)
    LightSpl,
    /// T22 mint + Light user accounts (offchain T22->Light conversion)
    LightT22,
    /// Light mint + Light user accounts (pure Light-to-Light)
    Light,
}

impl TokenConfig {
    pub fn mint_type(&self) -> MintType {
        match self {
            TokenConfig::Spl | TokenConfig::LightSpl => MintType::Spl,
            TokenConfig::Token2022 | TokenConfig::LightT22 => MintType::Token2022,
            TokenConfig::Light => MintType::Spl, // Light mints use SPL-compatible layout
        }
    }

    pub fn token_program_id(&self) -> Pubkey {
        match self {
            TokenConfig::Spl | TokenConfig::LightSpl => token::ID,
            TokenConfig::Token2022 | TokenConfig::LightT22 => spl_token_2022::ID,
            TokenConfig::Light => Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        }
    }

    /// Returns true if user accounts are Light token accounts
    pub fn uses_light_user_accounts(&self) -> bool {
        matches!(
            self,
            TokenConfig::LightSpl | TokenConfig::LightT22 | TokenConfig::Light
        )
    }

    /// Returns true if mints are Light Protocol mints (not SPL/T22)
    pub fn uses_light_mints(&self) -> bool {
        matches!(self, TokenConfig::Light)
    }
}

/// Context for AMM tests containing all necessary accounts
pub struct AmmTestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub mint_a_pubkey: Pubkey,
    pub mint_b_pubkey: Pubkey,
    pub amm_pda: Pubkey,
    pub amm_id: Pubkey,
    pub pool_pda: Pubkey,
    pub pool_authority: Pubkey,
    pub mint_liquidity: Pubkey,
    pub pool_account_a: Pubkey,
    pub pool_account_b: Pubkey,
    pub pool_a_bump: u8,
    pub pool_b_bump: u8,
    pub spl_interface_pda_a: Pubkey,
    pub spl_interface_pda_b: Pubkey,
    pub spl_interface_bump_a: u8,
    pub depositor: Keypair,
    pub depositor_ata_a: Pubkey,
    pub depositor_ata_b: Pubkey,
    pub token_config: TokenConfig,
    /// Compression config PDA
    pub compression_config: Pubkey,
    /// Authority keypair for Light mint A (if Light config)
    pub light_mint_authority_a: Option<Keypair>,
}

/// Create a new LightProgramTest instance for token-swap tests
pub async fn create_test_rpc() -> LightProgramTest {
    let program_id = swap_example::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![
            ("swap_example", program_id),
            ("light_token_minter", LIGHT_TOKEN_MINTER_PROGRAM_ID),
        ]),
    );
    config = config.with_light_protocol_events();
    LightProgramTest::new(config).await.unwrap()
}

/// Setup the AMM test environment based on token config
pub async fn setup_amm_test<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    config: TokenConfig,
) -> AmmTestContext {
    let program_id = swap_example::ID;
    let payer = rpc.get_payer().insecure_clone();

    // Initialize rent-free config (returns the config PDA)
    let rent_sponsor = swap_example::program_rent_sponsor();
    let compression_config = initialize_rent_free_config(rpc, &payer, &program_id, rent_sponsor).await;

    // Fund the program rent sponsor PDA
    rpc.airdrop_lamports(&rent_sponsor, 1_000_000_000)
        .await
        .unwrap();

    // Create depositor
    let depositor = Keypair::new();
    rpc.airdrop_lamports(&depositor.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Token amount for depositor
    let amount = 10_000_000_000_000u64; // 10,000 tokens with 9 decimals

    // For Light mints, we need to handle things differently
    if config.uses_light_mints() {
        // ========== LIGHT MINT SETUP ==========
        println!("\n=== Setting up Light mints ===");

        // Create Light mints
        let light_mint_a =
            create_light_mint(rpc, &payer, 9, "Token A", "TOKA", &compression_config).await;
        let light_mint_b =
            create_light_mint(rpc, &payer, 9, "Token B", "TOKB", &compression_config).await;

        let mint_a_pubkey = light_mint_a.mint;
        let mint_b_pubkey = light_mint_b.mint;

        println!("Light Mint A: {:?}", mint_a_pubkey);
        println!("Light Mint B: {:?}", mint_b_pubkey);

        // For pure Light-to-Light, we don't need SPL interface PDAs
        // Use dummy pubkeys (these won't be used)
        let spl_interface_pda_a = Pubkey::default();
        let spl_interface_pda_b = Pubkey::default();

        // Mint Light tokens directly to depositor
        println!("\n=== Minting Light tokens to depositor ===");
        let depositor_ata_a = mint_light_tokens(
            rpc,
            &payer,
            &light_mint_a.authority,
            &mint_a_pubkey,
            &depositor.pubkey(),
            amount,
        )
        .await;
        let depositor_ata_b = mint_light_tokens(
            rpc,
            &payer,
            &light_mint_b.authority,
            &mint_b_pubkey,
            &depositor.pubkey(),
            amount,
        )
        .await;

        // Derive AMM PDA
        let amm_id = Pubkey::new_unique();
        let (amm_pda, _) = Pubkey::find_program_address(&[amm_id.as_ref()], &program_id);

        // Derive pool PDAs using Light mint pubkeys
        let (pool_pda, _) = Pubkey::find_program_address(
            &[
                amm_pda.as_ref(),
                mint_a_pubkey.as_ref(),
                mint_b_pubkey.as_ref(),
            ],
            &program_id,
        );

        let (pool_authority, _) = Pubkey::find_program_address(
            &[b"authority"],
            &program_id,
        );

        let (mint_liquidity, _) = Pubkey::find_program_address(
            &[
                amm_pda.as_ref(),
                mint_a_pubkey.as_ref(),
                mint_b_pubkey.as_ref(),
                b"liquidity",
            ],
            &program_id,
        );

        let (pool_account_a, pool_a_bump) =
            Pubkey::find_program_address(&[b"pool_a", pool_pda.as_ref()], &program_id);
        let (pool_account_b, pool_b_bump) =
            Pubkey::find_program_address(&[b"pool_b", pool_pda.as_ref()], &program_id);

        return AmmTestContext {
            program_id,
            payer,
            mint_a_pubkey,
            mint_b_pubkey,
            amm_pda,
            amm_id,
            pool_pda,
            pool_authority,
            mint_liquidity,
            pool_account_a,
            pool_account_b,
            pool_a_bump,
            pool_b_bump,
            spl_interface_pda_a,
            spl_interface_pda_b,
            spl_interface_bump_a: 0,
            depositor,
            depositor_ata_a,
            depositor_ata_b,
            token_config: config,
            compression_config,
            light_mint_authority_a: Some(light_mint_a.authority),
        };
    }

    // ========== SPL/T22 MINT SETUP ==========
    // Create mints based on config (SPL or T22 - Light user accounts still use SPL/T22 mints)
    let (mint_a, mint_b) = match config {
        TokenConfig::Spl | TokenConfig::LightSpl => {
            let mint_a = create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await;
            let mint_b = create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await;
            (mint_a, mint_b)
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            let mint_a = create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await;
            let mint_b = create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await;
            (mint_a, mint_b)
        }
        TokenConfig::Light => unreachable!("Light config handled above"),
    };

    let mint_a_pubkey = mint_a.pubkey();
    let mint_b_pubkey = mint_b.pubkey();

    println!("Mint A: {:?}", mint_a_pubkey);
    println!("Mint B: {:?}", mint_b_pubkey);

    // Create SPL interface PDAs for both mints
    println!("\n=== Creating SPL interface PDAs ===");
    let spl_interface_result_a =
        create_spl_interface_pda(rpc, &payer, &mint_a_pubkey, config.mint_type(), false).await;
    let spl_interface_result_b =
        create_spl_interface_pda(rpc, &payer, &mint_b_pubkey, config.mint_type(), false).await;

    // Create depositor ATAs and mint tokens based on config
    let (depositor_ata_a, depositor_ata_b) = match config {
        TokenConfig::Spl => {
            let ata_a = create_spl_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let ata_b = create_spl_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_spl_tokens(rpc, &payer, &mint_a_pubkey, &ata_a, &payer, amount).await;
            mint_spl_tokens(rpc, &payer, &mint_b_pubkey, &ata_b, &payer, amount).await;
            (ata_a, ata_b)
        }
        TokenConfig::Token2022 => {
            let ata_a = create_t22_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let ata_b = create_t22_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_t22_tokens(rpc, &payer, &mint_a_pubkey, &ata_a, &payer, amount).await;
            mint_t22_tokens(rpc, &payer, &mint_b_pubkey, &ata_b, &payer, amount).await;
            (ata_a, ata_b)
        }
        TokenConfig::LightSpl => {
            // For Light user accounts with SPL mint:
            // 1. Create temporary SPL ATAs
            // 2. Mint tokens to temp ATAs
            // 3. Create Light ATAs
            // 4. Transfer from SPL to Light (compress)
            println!("\n=== Setting up Light user accounts (SPL mint) ===");

            let temp_ata_a = create_spl_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let temp_ata_b = create_spl_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_spl_tokens(rpc, &payer, &mint_a_pubkey, &temp_ata_a, &payer, amount).await;
            mint_spl_tokens(rpc, &payer, &mint_b_pubkey, &temp_ata_b, &payer, amount).await;

            // Create Light ATAs
            let light_ata_a =
                create_light_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;

            // Transfer from SPL to Light (compress)
            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &spl_interface_result_a.pda,
                spl_interface_result_a.bump,
                amount,
                MintType::Spl,
            )
            .await;
            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_b_pubkey,
                9,
                &temp_ata_b,
                &light_ata_b,
                &spl_interface_result_b.pda,
                spl_interface_result_b.bump,
                amount,
                MintType::Spl,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::LightT22 => {
            // For Light user accounts with T22 mint:
            println!("\n=== Setting up Light user accounts (T22 mint) ===");

            let temp_ata_a = create_t22_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let temp_ata_b = create_t22_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_t22_tokens(rpc, &payer, &mint_a_pubkey, &temp_ata_a, &payer, amount).await;
            mint_t22_tokens(rpc, &payer, &mint_b_pubkey, &temp_ata_b, &payer, amount).await;

            // Create Light ATAs
            let light_ata_a =
                create_light_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;

            // Transfer from T22 to Light (compress)
            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &spl_interface_result_a.pda,
                spl_interface_result_a.bump,
                amount,
                MintType::Token2022,
            )
            .await;
            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_b_pubkey,
                9,
                &temp_ata_b,
                &light_ata_b,
                &spl_interface_result_b.pda,
                spl_interface_result_b.bump,
                amount,
                MintType::Token2022,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::Light => unreachable!("Light config handled above"),
    };

    // Derive AMM PDA
    let amm_id = Pubkey::new_unique();
    let (amm_pda, _) = Pubkey::find_program_address(&[amm_id.as_ref()], &program_id);

    // Derive pool PDAs
    let (pool_pda, _) = Pubkey::find_program_address(
        &[
            amm_pda.as_ref(),
            mint_a_pubkey.as_ref(),
            mint_b_pubkey.as_ref(),
        ],
        &program_id,
    );

    let (pool_authority, _) = Pubkey::find_program_address(
        &[b"authority"],
        &program_id,
    );

    let (mint_liquidity, _) = Pubkey::find_program_address(
        &[
            amm_pda.as_ref(),
            mint_a_pubkey.as_ref(),
            mint_b_pubkey.as_ref(),
            b"liquidity",
        ],
        &program_id,
    );

    let (pool_account_a, pool_a_bump) =
        Pubkey::find_program_address(&[b"pool_a", pool_pda.as_ref()], &program_id);
    let (pool_account_b, pool_b_bump) =
        Pubkey::find_program_address(&[b"pool_b", pool_pda.as_ref()], &program_id);

    AmmTestContext {
        program_id,
        payer,
        mint_a_pubkey,
        mint_b_pubkey,
        amm_pda,
        amm_id,
        pool_pda,
        pool_authority,
        mint_liquidity,
        pool_account_a,
        pool_account_b,
        pool_a_bump,
        pool_b_bump,
        spl_interface_pda_a: spl_interface_result_a.pda,
        spl_interface_pda_b: spl_interface_result_b.pda,
        spl_interface_bump_a: spl_interface_result_a.bump,
        depositor,
        depositor_ata_a,
        depositor_ata_b,
        token_config: config,
        compression_config,
        light_mint_authority_a: None,
    }
}

/// Create AMM
pub async fn create_amm<R: Rpc>(rpc: &mut R, ctx: &AmmTestContext, fee: u16) {
    println!("\n=== Creating AMM ===");

    let create_amm_accounts = swap_example::accounts::CreateAmm {
        amm: ctx.amm_pda,
        admin: ctx.payer.pubkey(),
        payer: ctx.payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };

    let create_amm_data = swap_example::instruction::CreateAmm {
        id: ctx.amm_id,
        fee,
    };

    let create_amm_ix = Instruction {
        program_id: ctx.program_id,
        accounts: create_amm_accounts.to_account_metas(None),
        data: create_amm_data.data(),
    };

    rpc.create_and_send_transaction(&[create_amm_ix], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("create_amm should succeed");

    println!("AMM created at: {:?}", ctx.amm_pda);
    println!("AMM Fee: {} basis points", fee);
}

/// Create Pool with Light token accounts
pub async fn create_pool<R: Rpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    proof_result: CreateAccountsProofResult,
) {
    println!("\n=== Creating Pool ===");

    // For liquidity mint, use SPL for Light pools (can't init Light mints via Anchor)
    // For SPL/T22 pools, use the same token program as the mints
    let liquidity_token_program = match ctx.token_config {
        TokenConfig::Light | TokenConfig::LightSpl => token::ID,
        TokenConfig::Token2022 | TokenConfig::LightT22 => spl_token_2022::ID,
        TokenConfig::Spl => token::ID,
    };

    let pda_rent_sponsor = swap_example::program_rent_sponsor();

    let create_pool_accounts = swap_example::accounts::CreatePool {
        amm: ctx.amm_pda,
        pool: ctx.pool_pda,
        pool_authority: ctx.pool_authority,
        mint_liquidity: ctx.mint_liquidity,
        mint_a: ctx.mint_a_pubkey,
        mint_b: ctx.mint_b_pubkey,
        pool_account_a: ctx.pool_account_a,
        pool_account_b: ctx.pool_account_b,
        fee_payer: ctx.payer.pubkey(),
        token_program: ctx.token_config.token_program_id(),
        liquidity_token_program,
        system_program: solana_sdk::system_program::ID,
        compression_config: ctx.compression_config,
        pda_rent_sponsor,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
    };

    let create_pool_data = swap_example::instruction::CreatePool {
        params: swap_example::CreatePoolParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            pool_account_a_bump: ctx.pool_a_bump,
            pool_account_b_bump: ctx.pool_b_bump,
        },
    };

    let create_pool_ix = Instruction {
        program_id: ctx.program_id,
        accounts: [
            create_pool_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: create_pool_data.data(),
    };

    rpc.create_and_send_transaction(&[create_pool_ix], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("create_pool should succeed");

    println!("Pool created successfully!");
}

/// Deposit liquidity
pub async fn deposit_liquidity<R: Rpc>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    amount_a: u64,
    amount_b: u64,
) -> Pubkey {
    println!("\n=== Depositing Liquidity ===");

    let token_program = ctx.token_config.token_program_id();

    // Liquidity mint is SPL for Light/LightSpl pools, T22 for T22/LightT22 pools, SPL for Spl pools
    let liquidity_token_program = match ctx.token_config {
        TokenConfig::Light | TokenConfig::LightSpl | TokenConfig::Spl => token::ID,
        TokenConfig::Token2022 | TokenConfig::LightT22 => spl_token_2022::ID,
    };

    // Create depositor's liquidity ATA (must use liquidity token program)
    let depositor_liquidity_ata = get_associated_token_address_with_program_id(
        &ctx.depositor.pubkey(),
        &ctx.mint_liquidity,
        &liquidity_token_program,
    );

    // Create the liquidity ATA first (since program no longer does init_if_needed)
    // Liquidity mint is always SPL or T22 (not Light) - Anchor can't init Light mints
    match ctx.token_config {
        TokenConfig::Spl | TokenConfig::LightSpl | TokenConfig::Light => {
            // SPL liquidity mint
            create_spl_ata(
                rpc,
                &ctx.payer,
                &ctx.mint_liquidity,
                &ctx.depositor.pubkey(),
            )
            .await;
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            // T22 liquidity mint
            create_t22_ata(
                rpc,
                &ctx.payer,
                &ctx.mint_liquidity,
                &ctx.depositor.pubkey(),
            )
            .await;
        }
    }

    let deposit_accounts = swap_example::accounts::DepositLiquidity {
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
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a: ctx.spl_interface_pda_a,
        spl_interface_pda_b: ctx.spl_interface_pda_b,
    };

    let deposit_data = swap_example::instruction::DepositLiquidity { amount_a, amount_b };

    let deposit_ix = Instruction {
        program_id: ctx.program_id,
        accounts: deposit_accounts.to_account_metas(None),
        data: deposit_data.data(),
    };

    rpc.create_and_send_transaction(
        &[deposit_ix],
        &ctx.payer.pubkey(),
        &[&ctx.payer, &ctx.depositor],
    )
    .await
    .expect("deposit_liquidity should succeed");

    println!("Deposited {} token A and {} token B", amount_a, amount_b);

    depositor_liquidity_ata
}

/// Perform swap A->B or B->A
pub async fn swap<R: Rpc>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    trader: &Keypair,
    trader_ata_a: Pubkey,
    trader_ata_b: Pubkey,
    swap_a: bool,
    input_amount: u64,
    min_output: u64,
) {
    let direction = if swap_a { "A->B" } else { "B->A" };
    println!("\n=== Swapping {} ===", direction);

    let token_program = ctx.token_config.token_program_id();

    let swap_accounts = swap_example::accounts::SwapExactTokensForTokens {
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
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a: ctx.spl_interface_pda_a,
        spl_interface_pda_b: ctx.spl_interface_pda_b,
    };

    let swap_data = swap_example::instruction::SwapExactTokensForTokens {
        swap_a,
        input_amount,
        min_output_amount: min_output,
    };

    let swap_ix = Instruction {
        program_id: ctx.program_id,
        accounts: swap_accounts.to_account_metas(None),
        data: swap_data.data(),
    };

    rpc.create_and_send_transaction(&[swap_ix], &ctx.payer.pubkey(), &[&ctx.payer, trader])
        .await
        .expect(&format!("swap {} should succeed", direction));

    println!("Swapped {} of input token", input_amount);
}

/// Withdraw liquidity
pub async fn withdraw_liquidity<R: Rpc>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    depositor_liquidity_ata: Pubkey,
    amount: u64,
) {
    println!("\n=== Withdrawing Liquidity ===");

    let token_program = ctx.token_config.token_program_id();

    // Liquidity mint is SPL for Light/LightSpl pools, T22 for T22/LightT22 pools, SPL for Spl pools
    let liquidity_token_program = match ctx.token_config {
        TokenConfig::Light | TokenConfig::LightSpl | TokenConfig::Spl => token::ID,
        TokenConfig::Token2022 | TokenConfig::LightT22 => spl_token_2022::ID,
    };

    let withdraw_accounts = swap_example::accounts::WithdrawLiquidity {
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
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a: ctx.spl_interface_pda_a,
        spl_interface_pda_b: ctx.spl_interface_pda_b,
    };

    let withdraw_data = swap_example::instruction::WithdrawLiquidity { amount };

    let withdraw_ix = Instruction {
        program_id: ctx.program_id,
        accounts: withdraw_accounts.to_account_metas(None),
        data: withdraw_data.data(),
    };

    rpc.create_and_send_transaction(
        &[withdraw_ix],
        &ctx.payer.pubkey(),
        &[&ctx.payer, &ctx.depositor],
    )
    .await
    .expect("withdraw_liquidity should succeed");

    println!("Withdrew {} LP tokens", amount);
}

/// Create a trader with token accounts and funded tokens
pub async fn create_trader<R: Rpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    initial_a: u64,
) -> (Keypair, Pubkey, Pubkey) {
    let trader = Keypair::new();
    rpc.airdrop_lamports(&trader.pubkey(), 5_000_000_000)
        .await
        .unwrap();

    let (trader_ata_a, trader_ata_b) = match ctx.token_config {
        TokenConfig::Spl => {
            let ata_a = create_spl_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let ata_b = create_spl_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;
            // Mint initial token A to trader
            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;
            (ata_a, ata_b)
        }
        TokenConfig::Token2022 => {
            let ata_a = create_t22_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let ata_b = create_t22_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;
            // Mint initial token A to trader
            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;
            (ata_a, ata_b)
        }
        TokenConfig::LightSpl => {
            // For Light user accounts with SPL mint:
            // 1. Create temp SPL ATAs, mint, then transfer to Light ATAs
            let temp_ata_a =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let _temp_ata_b =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            // Mint initial token A to trader temp ATA
            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &temp_ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;

            // Create Light ATAs
            let light_ata_a =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            // Transfer from SPL to Light (compress)
            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &trader,
                &ctx.mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &ctx.spl_interface_pda_a,
                ctx.spl_interface_bump_a,
                initial_a,
                MintType::Spl,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::LightT22 => {
            // For Light user accounts with T22 mint:
            let temp_ata_a =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let _temp_ata_b =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            // Mint initial token A to trader temp ATA
            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &temp_ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;

            // Create Light ATAs
            let light_ata_a =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            // Transfer from T22 to Light (compress)
            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &trader,
                &ctx.mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &ctx.spl_interface_pda_a,
                ctx.spl_interface_bump_a,
                initial_a,
                MintType::Token2022,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::Light => {
            // For pure Light mints:
            // Mint directly to trader's Light ATAs using the Light mint authority
            let mint_authority_a = ctx
                .light_mint_authority_a
                .as_ref()
                .expect("Light config should have mint authority A");

            // Mint token A to trader
            let light_ata_a = mint_light_tokens(
                rpc,
                &ctx.payer,
                mint_authority_a,
                &ctx.mint_a_pubkey,
                &trader.pubkey(),
                initial_a,
            )
            .await;

            // Create Light ATA for token B (no minting needed initially)
            let light_ata_b =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            (light_ata_a, light_ata_b)
        }
    };

    (trader, trader_ata_a, trader_ata_b)
}

/// Get token balance from SPL/T22 account
pub async fn get_token_balance<R: Rpc>(rpc: &mut R, account: Pubkey) -> u64 {
    let account_data = rpc
        .get_account(account)
        .await
        .unwrap()
        .expect("Token account should exist");

    let token_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&account_data.data[..165]).unwrap();
    u64::from(token_state.amount)
}

/// Run the full AMM flow test
pub async fn run_amm_full_flow<R: Rpc + Indexer>(rpc: &mut R, ctx: &AmmTestContext) {
    use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};

    // Create AMM
    create_amm(rpc, ctx, 250).await; // 2.5% fee

    // Get proof for creating pool Light token accounts
    let proof_result = get_create_accounts_proof(
        rpc,
        &ctx.program_id,
        vec![
            CreateAccountsProofInput::pda(ctx.pool_account_a),
            CreateAccountsProofInput::pda(ctx.pool_account_b),
        ],
    )
    .await
    .unwrap();

    // Create Pool
    create_pool(rpc, ctx, proof_result).await;

    // Fund pool_authority for Light token transfers (rent top-ups)
    // The pool_authority needs lamports to pay for rent top-ups when transferring
    // from pool accounts to user accounts during swap/withdraw operations
    if ctx.token_config.uses_light_user_accounts() {
        rpc.airdrop_lamports(&ctx.pool_authority, 1_000_000_000)
            .await
            .expect("Fund pool_authority for rent top-ups");
        println!("Funded pool_authority with 1 SOL for rent top-ups");
    }

    // Verify initial pool balances
    verify_light_token_balance(rpc, ctx.pool_account_a, 0, "pool_account_a (initial)").await;
    verify_light_token_balance(rpc, ctx.pool_account_b, 0, "pool_account_b (initial)").await;

    // Deposit initial liquidity
    let deposit_amount = 1_000_000_000u64; // 1 token
    let depositor_liquidity_ata = deposit_liquidity(rpc, ctx, deposit_amount, deposit_amount).await;

    // Verify pool balances after deposit
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

    // Create trader and perform swaps
    let trader_initial_a = 100_000_000u64; // 0.1 tokens
    let (trader, trader_ata_a, trader_ata_b) = create_trader(rpc, ctx, trader_initial_a).await;

    // Swap A -> B
    let swap_input = 10_000_000u64; // 0.01 tokens
    swap(
        rpc,
        ctx,
        &trader,
        trader_ata_a,
        trader_ata_b,
        true,
        swap_input,
        1,
    )
    .await;

    // Verify trader got some token B
    let trader_b_balance = get_token_balance(rpc, trader_ata_b).await;
    assert!(trader_b_balance > 0, "Trader should have received token B");
    println!("Trader token B after swap: {}", trader_b_balance);

    // Swap B -> A (swap half back)
    let swap_b_input = trader_b_balance / 2;
    swap(
        rpc,
        ctx,
        &trader,
        trader_ata_a,
        trader_ata_b,
        false,
        swap_b_input,
        1,
    )
    .await;

    // Withdraw half of LP tokens
    let lp_balance = get_token_balance(rpc, depositor_liquidity_ata).await;
    let withdraw_amount = lp_balance / 2;
    withdraw_liquidity(rpc, ctx, depositor_liquidity_ata, withdraw_amount).await;

    // Verify LP tokens were burned
    let lp_balance_after = get_token_balance(rpc, depositor_liquidity_ata).await;
    assert_eq!(
        lp_balance_after,
        lp_balance - withdraw_amount,
        "LP tokens should be burned"
    );

    println!("\n=== AMM full flow test completed successfully! ===");
}
