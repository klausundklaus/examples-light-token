//! Common test utilities for fundraiser tests.
//!
//! This module provides shared test helpers for different token type combinations:
//! - SPL mint + SPL ATAs
//! - T22 mint + T22 ATAs
//! - SPL mint + Light user accounts
//! - T22 mint + Light user accounts
//! - Light mint + Light user accounts

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::token;
use shared_test_utils::{
    helpers::verify_light_token_balance,
    light_tokens::{create_light_ata, create_light_mint, mint_light_tokens},
    setup::initialize_rent_free_config,
    spl_interface::{create_spl_interface_pda, transfer_spl_to_light},
    spl_tokens::{create_spl_ata, create_spl_mint, mint_spl_tokens},
    t22_tokens::{create_t22_ata, create_t22_mint, mint_t22_tokens},
    CreateAccountsProofResult, Indexer, LightProgramTest, MintType, ProgramTestConfig, Rpc,
    TestRpc, COMPRESSIBLE_CONFIG_V1, CPI_AUTHORITY_PDA, LIGHT_TOKEN_MINTER_PROGRAM_ID,
    LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR,
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

    /// Returns true if mints are Light Protocol mints (not SPL/T22)
    pub fn uses_light_mints(&self) -> bool {
        matches!(self, TokenConfig::Light)
    }
}

/// Context for fundraiser tests containing all necessary accounts
#[allow(dead_code)]
pub struct FundraiserTestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub mint: Keypair,
    /// For Light mints, stores the mint PDA
    pub mint_pubkey: Pubkey,
    pub maker: Keypair,
    pub fundraiser_pda: Pubkey,
    pub vault_pda: Pubkey,
    pub vault_bump: u8,
    pub spl_interface_pda: Pubkey,
    pub spl_interface_bump: u8,
    pub token_config: TokenConfig,
    pub amount_to_raise: u64,
    pub duration: u16,
    /// Compression config PDA (needed for Light mint creation)
    pub compression_config: Option<Pubkey>,
    /// Authority keypair for Light mint (if Light config)
    pub light_mint_authority: Option<Keypair>,
}

/// Create a new LightProgramTest instance for fundraiser tests
pub async fn create_test_rpc() -> LightProgramTest {
    let program_id = fundraiser::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![
            ("fundraiser", program_id),
            ("light_token_minter", LIGHT_TOKEN_MINTER_PROGRAM_ID),
        ]),
    );
    config = config.with_light_protocol_events();
    LightProgramTest::new(config).await.unwrap()
}

/// Setup the fundraiser test environment based on token config
pub async fn setup_fundraiser_test<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    config: TokenConfig,
) -> FundraiserTestContext {
    let program_id = fundraiser::ID;
    let payer = rpc.get_payer().insecure_clone();

    // Initialize rent-free config (returns the config PDA)
    let compression_config = initialize_rent_free_config(rpc, &payer, &program_id).await;

    // Create maker
    let maker = Keypair::new();
    rpc.airdrop_lamports(&maker.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Amount to raise: 1000 tokens (with 9 decimals)
    let amount_to_raise = 1_000_000_000_000u64;
    let duration = 7u16; // 7 days

    // For Light mints, handle separately
    if config.uses_light_mints() {
        // ========== LIGHT MINT SETUP ==========
        println!("\n=== Setting up Light mint ===");

        // Create Light mint
        let light_mint = create_light_mint(
            rpc,
            &payer,
            9,
            "Fundraiser Token",
            "FUND",
            &compression_config,
        )
        .await;

        let mint_pubkey = light_mint.mint;
        println!("Light Mint: {:?}", mint_pubkey);

        // For pure Light-to-Light, we don't need SPL interface PDA
        let spl_interface_pda = Pubkey::default();

        // Derive fundraiser and vault PDAs
        let (fundraiser_pda, _) =
            Pubkey::find_program_address(&[b"fundraiser", maker.pubkey().as_ref()], &program_id);

        let (vault_pda, vault_bump) = Pubkey::find_program_address(
            &[fundraiser::VAULT_SEED, fundraiser_pda.as_ref()],
            &program_id,
        );

        println!("Fundraiser PDA: {:?}", fundraiser_pda);
        println!("Vault PDA: {:?}", vault_pda);

        // Create dummy keypair for mint (not used for Light mints)
        let dummy_keypair = Keypair::new();

        return FundraiserTestContext {
            program_id,
            payer,
            mint: dummy_keypair,
            mint_pubkey,
            maker,
            fundraiser_pda,
            vault_pda,
            vault_bump,
            spl_interface_pda,
            spl_interface_bump: 0,
            token_config: config,
            amount_to_raise,
            duration,
            compression_config: Some(compression_config),
            light_mint_authority: Some(light_mint.authority),
        };
    }

    // ========== SPL/T22 MINT SETUP ==========
    // Create mint based on config (SPL or T22 - Light user accounts still use SPL/T22 mints)
    let mint = match config {
        TokenConfig::Spl | TokenConfig::LightSpl => {
            create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await
        }
        TokenConfig::Light => unreachable!("Light config handled above"),
    };

    let mint_pubkey = mint.pubkey();
    println!("Mint created: {:?}", mint_pubkey);

    // Create SPL interface PDA
    println!("\n=== Creating SPL interface PDA ===");
    let spl_interface_result =
        create_spl_interface_pda(rpc, &payer, &mint_pubkey, config.mint_type(), false).await;

    // Derive fundraiser and vault PDAs
    let (fundraiser_pda, _) =
        Pubkey::find_program_address(&[b"fundraiser", maker.pubkey().as_ref()], &program_id);

    let (vault_pda, vault_bump) = Pubkey::find_program_address(
        &[fundraiser::VAULT_SEED, fundraiser_pda.as_ref()],
        &program_id,
    );

    println!("Fundraiser PDA: {:?}", fundraiser_pda);
    println!("Vault PDA: {:?}", vault_pda);

    FundraiserTestContext {
        program_id,
        payer,
        mint,
        mint_pubkey,
        maker,
        fundraiser_pda,
        vault_pda,
        vault_bump,
        spl_interface_pda: spl_interface_result.pda,
        spl_interface_bump: spl_interface_result.bump,
        token_config: config,
        amount_to_raise,
        duration,
        compression_config: Some(compression_config),
        light_mint_authority: None,
    }
}

/// Initialize fundraiser
pub async fn initialize_fundraiser<R: Rpc + Indexer>(
    rpc: &mut R,
    ctx: &FundraiserTestContext,
    proof_result: CreateAccountsProofResult,
) {
    println!("\n=== Initializing Fundraiser ===");

    let token_program = ctx.token_config.token_program_id();

    let initialize_accounts = fundraiser::accounts::Initialize {
        fee_payer: ctx.maker.pubkey(),
        mint_to_raise: ctx.mint_pubkey,
        fundraiser: ctx.fundraiser_pda,
        vault: ctx.vault_pda,
        token_program,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
    };

    let initialize_data = fundraiser::instruction::Initialize {
        params: fundraiser::instructions::InitializeParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            amount: ctx.amount_to_raise,
            duration: ctx.duration,
            vault_bump: ctx.vault_bump,
        },
    };

    let initialize_ix = Instruction {
        program_id: ctx.program_id,
        accounts: [
            initialize_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: initialize_data.data(),
    };

    rpc.create_and_send_transaction(
        &[initialize_ix],
        &ctx.payer.pubkey(),
        &[&ctx.payer, &ctx.maker],
    )
    .await
    .expect("initialize should succeed");

    println!("Fundraiser initialized!");
    println!(
        "  Amount to raise: {} tokens",
        ctx.amount_to_raise / 1_000_000_000
    );
    println!("  Duration: {} days", ctx.duration);
}

/// Create a contributor with ATA and funded tokens
pub async fn create_contributor<R: Rpc + Indexer>(
    rpc: &mut R,
    ctx: &FundraiserTestContext,
    funding_amount: u64,
) -> (Keypair, Pubkey) {
    let contributor = Keypair::new();
    rpc.airdrop_lamports(&contributor.pubkey(), 5_000_000_000)
        .await
        .unwrap();

    let contributor_ata = match ctx.token_config {
        TokenConfig::Spl => {
            let ata =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &ata,
                &ctx.payer,
                funding_amount,
            )
            .await;
            ata
        }
        TokenConfig::Token2022 => {
            let ata =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &ata,
                &ctx.payer,
                funding_amount,
            )
            .await;
            ata
        }
        TokenConfig::LightSpl => {
            // For Light user accounts with SPL mint:
            // 1. Create temp SPL ATA, mint tokens
            // 2. Create Light ATA
            // 3. Transfer from SPL to Light (compress)
            println!("Creating Light contributor account (SPL mint)");

            let temp_ata =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &temp_ata,
                &ctx.payer,
                funding_amount,
            )
            .await;

            // Create Light ATA
            let light_ata =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;

            // Transfer from SPL to Light (compress)
            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &contributor,
                &ctx.mint_pubkey,
                9,
                &temp_ata,
                &light_ata,
                &ctx.spl_interface_pda,
                ctx.spl_interface_bump,
                funding_amount,
                MintType::Spl,
            )
            .await;

            light_ata
        }
        TokenConfig::LightT22 => {
            // For Light user accounts with T22 mint:
            println!("Creating Light contributor account (T22 mint)");

            let temp_ata =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &temp_ata,
                &ctx.payer,
                funding_amount,
            )
            .await;

            // Create Light ATA
            let light_ata =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;

            // Transfer from T22 to Light (compress)
            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &contributor,
                &ctx.mint_pubkey,
                9,
                &temp_ata,
                &light_ata,
                &ctx.spl_interface_pda,
                ctx.spl_interface_bump,
                funding_amount,
                MintType::Token2022,
            )
            .await;

            light_ata
        }
        TokenConfig::Light => {
            // For pure Light mints:
            // Mint directly to contributor's Light ATA
            println!("Creating Light contributor account (Light mint)");

            let mint_authority = ctx
                .light_mint_authority
                .as_ref()
                .expect("Light config should have mint authority");

            // Mint tokens directly to contributor's Light ATA
            let light_ata = mint_light_tokens(
                rpc,
                &ctx.payer,
                mint_authority,
                &ctx.mint_pubkey,
                &contributor.pubkey(),
                funding_amount,
            )
            .await;

            light_ata
        }
    };

    (contributor, contributor_ata)
}

/// Contribute to fundraiser
pub async fn contribute<R: Rpc>(
    rpc: &mut R,
    ctx: &FundraiserTestContext,
    contributor: &Keypair,
    contributor_ata: Pubkey,
    amount: u64,
) {
    println!("\n=== Contributing ===");

    let token_program = ctx.token_config.token_program_id();

    // Derive contributor account PDA
    let (contributor_account_pda, _) = Pubkey::find_program_address(
        &[
            b"contributor",
            ctx.fundraiser_pda.as_ref(),
            contributor.pubkey().as_ref(),
        ],
        &ctx.program_id,
    );

    let contribute_accounts = fundraiser::accounts::Contribute {
        contributor: contributor.pubkey(),
        mint_to_raise: ctx.mint_pubkey,
        fundraiser: ctx.fundraiser_pda,
        contributor_account: contributor_account_pda,
        contributor_ata,
        vault: ctx.vault_pda,
        token_program,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda: ctx.spl_interface_pda,
    };

    let contribute_data = fundraiser::instruction::Contribute { amount };

    let contribute_ix = Instruction {
        program_id: ctx.program_id,
        accounts: contribute_accounts.to_account_metas(None),
        data: contribute_data.data(),
    };

    rpc.create_and_send_transaction(
        &[contribute_ix],
        &ctx.payer.pubkey(),
        &[&ctx.payer, contributor],
    )
    .await
    .expect("contribute should succeed");

    println!("Contributed {} tokens", amount / 1_000_000_000);
}

/// Check contributions and claim funds (maker claims)
pub async fn check_contributions<R: Rpc>(rpc: &mut R, ctx: &FundraiserTestContext) -> Pubkey {
    println!("\n=== Maker Claims Funds ===");

    let token_program = ctx.token_config.token_program_id();

    // Create maker's ATA (SPL/T22 receives from Light vault via SPL interface, Light receives directly)
    let maker_ata = match ctx.token_config {
        TokenConfig::Spl | TokenConfig::LightSpl => {
            create_spl_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &ctx.maker.pubkey()).await
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            create_t22_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &ctx.maker.pubkey()).await
        }
        TokenConfig::Light => {
            // For pure Light mints, maker receives to Light ATA
            create_light_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &ctx.maker.pubkey()).await
        }
    };

    let check_contributions_accounts = fundraiser::accounts::CheckContributions {
        fee_payer: ctx.maker.pubkey(),
        mint_to_raise: ctx.mint_pubkey,
        fundraiser: ctx.fundraiser_pda,
        vault: ctx.vault_pda,
        maker_ata,
        token_program,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda: ctx.spl_interface_pda,
    };

    let check_contributions_data = fundraiser::instruction::CheckContributions {};

    let check_contributions_ix = Instruction {
        program_id: ctx.program_id,
        accounts: check_contributions_accounts.to_account_metas(None),
        data: check_contributions_data.data(),
    };

    rpc.create_and_send_transaction(
        &[check_contributions_ix],
        &ctx.payer.pubkey(),
        &[&ctx.payer, &ctx.maker],
    )
    .await
    .expect("check_contributions should succeed");

    println!("Maker claimed funds!");
    maker_ata
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

/// Run the full fundraiser flow test
pub async fn run_fundraiser_full_flow<R: Rpc + Indexer>(rpc: &mut R, ctx: &FundraiserTestContext) {
    use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};

    // Get proof for creating vault Light token account
    let proof_result = get_create_accounts_proof(
        rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(ctx.vault_pda)],
    )
    .await
    .unwrap();

    // Initialize fundraiser
    initialize_fundraiser(rpc, ctx, proof_result).await;

    // Verify vault was created with 0 balance
    verify_light_token_balance(rpc, ctx.vault_pda, 0, "vault (initial)").await;

    // Contribution amount is 10% of target (max allowed per contributor)
    let contribution_amount = ctx.amount_to_raise * 10 / 100;

    // We need 10 contributors to reach target (each contributes 10%)
    let num_contributors = 10;
    let contributor_funding = contribution_amount * 2; // Give them extra to be safe

    println!(
        "\n=== Creating {} contributors (each contributing {} tokens) ===",
        num_contributors,
        contribution_amount / 1_000_000_000
    );

    let mut total_contributed = 0u64;
    for i in 0..num_contributors {
        let (contributor, contributor_ata) =
            create_contributor(rpc, ctx, contributor_funding).await;
        contribute(rpc, ctx, &contributor, contributor_ata, contribution_amount).await;
        total_contributed += contribution_amount;
        println!(
            "Contributor {} contributed, total: {} tokens",
            i + 1,
            total_contributed / 1_000_000_000
        );
    }

    // Verify vault has reached target
    verify_light_token_balance(
        rpc,
        ctx.vault_pda,
        ctx.amount_to_raise,
        "vault (target reached)",
    )
    .await;

    // Maker claims funds
    let maker_ata = check_contributions(rpc, ctx).await;

    // Verify vault is empty
    verify_light_token_balance(rpc, ctx.vault_pda, 0, "vault (after claim)").await;

    // Verify maker received funds
    let maker_balance = get_token_balance(rpc, maker_ata).await;
    assert_eq!(
        maker_balance, ctx.amount_to_raise,
        "Maker should have received all raised funds"
    );
    println!("Maker received {} tokens", maker_balance / 1_000_000_000);

    // Verify fundraiser account was closed
    let fundraiser_account = rpc.get_account(ctx.fundraiser_pda).await.unwrap();
    assert!(fundraiser_account.is_none(), "Fundraiser should be closed");

    println!("\n=== Fundraiser full flow test completed successfully! ===");
}
