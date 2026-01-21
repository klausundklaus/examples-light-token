//! Integration tests for the fundraiser program with Light Protocol token accounts.
//!
//! This test demonstrates a full fundraising flow using:
//! - Standard SPL mint for the token to raise
//! - Standard SPL token accounts for contributors
//! - Light Protocol token account for the vault

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token;
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
use light_token::constants::CPI_AUTHORITY_PDA;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};
use light_token::spl_interface::{find_spl_interface_pda, CreateSplInterfacePda};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022::pod::PodAccount;

/// Verify Light Token account balance.
async fn verify_light_token_balance<R: Rpc>(rpc: &mut R, account: Pubkey, expected: u64, name: &str) {
    let account_data = rpc.get_account(account).await.unwrap();

    if let Some(data) = account_data {
        // Light Token accounts have first 165 bytes as SPL-compatible token account data
        if data.data.len() >= 165 {
            let token_state =
                spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&data.data[..165]).unwrap();
            let actual = u64::from(token_state.amount);
            println!("{}: balance = {} (expected {})", name, actual, expected);
            assert_eq!(
                actual, expected,
                "{} balance mismatch: expected {}, got {}",
                name, expected, actual
            );
        } else {
            panic!(
                "{}: account data too short: {} bytes",
                name,
                data.data.len()
            );
        }
    } else if expected == 0 {
        println!("{}: account not found (expected 0 balance)", name);
    } else {
        panic!(
            "{}: account not found but expected balance {}",
            name, expected
        );
    }
}

/// Test the full fundraiser flow: initialize, contribute, check_contributions
#[tokio::test]
async fn test_fundraiser_full_flow() {
    let program_id = fundraiser::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("fundraiser", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Setup program data for rent-free config
    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize rent-free config
    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    println!("Rent-free config initialized at: {:?}", config_pda);

    // === STEP 1: Create SPL mint for fundraising token ===
    println!("\n=== Creating SPL mint ===");

    let mint = Keypair::new();
    let decimals = 9u8;

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(82)
        .await
        .unwrap();

    let create_mint_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        82,
        &token::ID,
    );
    let init_mint_ix = token::spl_token::instruction::initialize_mint(
        &token::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint_ix, init_mint_ix],
        &payer.pubkey(),
        &[&payer, &mint],
    )
    .await
    .expect("Create mint should succeed");

    println!("Mint created: {:?}", mint.pubkey());

    // === STEP 1.5: Create SPL interface PDA for mint ===
    println!("\n=== Creating SPL interface PDA ===");

    let (spl_interface_pda, _) = find_spl_interface_pda(&mint.pubkey(), false);

    let create_spl_interface_ix =
        CreateSplInterfacePda::new(payer.pubkey(), mint.pubkey(), token::ID, false).instruction();

    rpc.create_and_send_transaction(&[create_spl_interface_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Create SPL interface PDA should succeed");

    println!("SPL interface PDA: {:?}", spl_interface_pda);

    // === STEP 2: Create maker (fundraiser creator) ===
    println!("\n=== Setting up maker ===");

    let maker = Keypair::new();
    rpc.airdrop_lamports(&maker.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Create maker's ATA for receiving funds after successful fundraising
    let maker_ata = get_associated_token_address(&maker.pubkey(), &mint.pubkey());

    println!("Maker: {:?}", maker.pubkey());
    println!("Maker ATA: {:?}", maker_ata);

    // === STEP 3: Initialize Fundraiser ===
    println!("\n=== Initializing Fundraiser ===");

    // Amount to raise: 1000 tokens (must be >= 3 * 10^decimals)
    let amount_to_raise = 1_000_000_000_000u64; // 1000 tokens with 9 decimals
    let duration = 7u16; // 7 days

    // Derive fundraiser PDA
    let (fundraiser_pda, _fundraiser_bump) = Pubkey::find_program_address(
        &[b"fundraiser", maker.pubkey().as_ref()],
        &program_id,
    );

    // Derive vault PDA (Light token account)
    let (vault_pda, vault_bump) = Pubkey::find_program_address(
        &[fundraiser::VAULT_SEED, fundraiser_pda.as_ref()],
        &program_id,
    );

    println!("Fundraiser PDA: {:?}", fundraiser_pda);
    println!("Vault PDA: {:?}", vault_pda);

    // Get proof for creating the vault Light token account
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(vault_pda)],
    )
    .await
    .unwrap();

    let initialize_accounts = fundraiser::accounts::Initialize {
        fee_payer: maker.pubkey(),
        mint_to_raise: mint.pubkey(),
        fundraiser: fundraiser_pda,
        vault: vault_pda,
        token_program: token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
    };

    let initialize_data = fundraiser::instruction::Initialize {
        params: fundraiser::instructions::InitializeParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            amount: amount_to_raise,
            duration,
            vault_bump,
        },
    };

    let initialize_ix = Instruction {
        program_id,
        accounts: [
            initialize_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: initialize_data.data(),
    };

    rpc.create_and_send_transaction(&[initialize_ix], &payer.pubkey(), &[&payer, &maker])
        .await
        .expect("initialize should succeed");

    println!("Fundraiser initialized!");
    println!("  Amount to raise: {} tokens", amount_to_raise / 1_000_000_000);
    println!("  Duration: {} days", duration);

    // Verify fundraiser state
    let fundraiser_account = rpc
        .get_account(fundraiser_pda)
        .await
        .unwrap()
        .expect("Fundraiser should exist");
    assert!(!fundraiser_account.data.is_empty(), "Fundraiser should have data");

    // Verify vault was created with 0 balance
    verify_light_token_balance(&mut rpc, vault_pda, 0, "vault (initial)").await;

    // === STEP 4: Create contributors and fund them ===
    println!("\n=== Setting up contributors ===");

    let contributor1 = Keypair::new();
    let contributor2 = Keypair::new();

    rpc.airdrop_lamports(&contributor1.pubkey(), 5_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&contributor2.pubkey(), 5_000_000_000)
        .await
        .unwrap();

    // Create contributor ATAs
    let contributor1_ata = get_associated_token_address(&contributor1.pubkey(), &mint.pubkey());
    let contributor2_ata = get_associated_token_address(&contributor2.pubkey(), &mint.pubkey());

    let create_ata1_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &contributor1.pubkey(),
            &mint.pubkey(),
            &token::ID,
        );
    let create_ata2_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &contributor2.pubkey(),
            &mint.pubkey(),
            &token::ID,
        );

    rpc.create_and_send_transaction(
        &[create_ata1_ix, create_ata2_ix],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .expect("Create contributor ATAs should succeed");

    // Mint tokens to contributors
    // Max contribution is 10% of amount_to_raise = 100 tokens
    let contributor_funding = 200_000_000_000u64; // 200 tokens each (enough for max contribution)

    let mint_to_1_ix = token::spl_token::instruction::mint_to(
        &token::ID,
        &mint.pubkey(),
        &contributor1_ata,
        &payer.pubkey(),
        &[],
        contributor_funding,
    )
    .unwrap();
    let mint_to_2_ix = token::spl_token::instruction::mint_to(
        &token::ID,
        &mint.pubkey(),
        &contributor2_ata,
        &payer.pubkey(),
        &[],
        contributor_funding,
    )
    .unwrap();

    rpc.create_and_send_transaction(&[mint_to_1_ix, mint_to_2_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Mint to contributors should succeed");

    println!("Contributor 1: {:?}", contributor1.pubkey());
    println!("Contributor 2: {:?}", contributor2.pubkey());

    // === STEP 5: First Contribution ===
    println!("\n=== First Contribution ===");

    // Contribute 10% of target (max allowed per contributor)
    let contribution_amount = amount_to_raise * 10 / 100; // 100 tokens

    // Derive contributor account PDA
    let (contributor1_account_pda, _) = Pubkey::find_program_address(
        &[
            b"contributor",
            fundraiser_pda.as_ref(),
            contributor1.pubkey().as_ref(),
        ],
        &program_id,
    );

    let contribute1_accounts = fundraiser::accounts::Contribute {
        contributor: contributor1.pubkey(),
        mint_to_raise: mint.pubkey(),
        fundraiser: fundraiser_pda,
        contributor_account: contributor1_account_pda,
        contributor_ata: contributor1_ata,
        vault: vault_pda,
        token_program: token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda,
    };

    let contribute1_data = fundraiser::instruction::Contribute {
        amount: contribution_amount,
    };

    let contribute1_ix = Instruction {
        program_id,
        accounts: contribute1_accounts.to_account_metas(None),
        data: contribute1_data.data(),
    };

    rpc.create_and_send_transaction(&[contribute1_ix], &payer.pubkey(), &[&payer, &contributor1])
        .await
        .expect("contribute should succeed");

    println!("Contributor 1 contributed {} tokens", contribution_amount / 1_000_000_000);

    // Verify vault balance
    verify_light_token_balance(&mut rpc, vault_pda, contribution_amount, "vault (after contribution 1)").await;

    // === STEP 6: Second Contribution ===
    println!("\n=== Second Contribution ===");

    // Derive contributor2 account PDA
    let (contributor2_account_pda, _) = Pubkey::find_program_address(
        &[
            b"contributor",
            fundraiser_pda.as_ref(),
            contributor2.pubkey().as_ref(),
        ],
        &program_id,
    );

    let contribute2_accounts = fundraiser::accounts::Contribute {
        contributor: contributor2.pubkey(),
        mint_to_raise: mint.pubkey(),
        fundraiser: fundraiser_pda,
        contributor_account: contributor2_account_pda,
        contributor_ata: contributor2_ata,
        vault: vault_pda,
        token_program: token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda,
    };

    let contribute2_data = fundraiser::instruction::Contribute {
        amount: contribution_amount,
    };

    let contribute2_ix = Instruction {
        program_id,
        accounts: contribute2_accounts.to_account_metas(None),
        data: contribute2_data.data(),
    };

    rpc.create_and_send_transaction(&[contribute2_ix], &payer.pubkey(), &[&payer, &contributor2])
        .await
        .expect("contribute should succeed");

    println!("Contributor 2 contributed {} tokens", contribution_amount / 1_000_000_000);

    // Verify vault balance (should be 2x contribution_amount now)
    verify_light_token_balance(&mut rpc, vault_pda, contribution_amount * 2, "vault (after contribution 2)").await;

    // === STEP 7: More contributions to reach target ===
    println!("\n=== Additional contributions to reach target ===");

    // We need 1000 tokens total, have 200 so far
    // Need 800 more, but max per contributor is 100
    // Create more contributors

    let remaining_needed = amount_to_raise - (contribution_amount * 2);
    let chunks_needed = (remaining_needed / contribution_amount) as usize;

    println!("Need {} more contributions of {} tokens each", chunks_needed, contribution_amount / 1_000_000_000);

    let mut total_contributed = contribution_amount * 2;

    for i in 0..chunks_needed {
        let contributor = Keypair::new();
        rpc.airdrop_lamports(&contributor.pubkey(), 5_000_000_000)
            .await
            .unwrap();

        // Create ATA
        let contributor_ata = get_associated_token_address(&contributor.pubkey(), &mint.pubkey());
        let create_ata_ix =
            anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                &contributor.pubkey(),
                &mint.pubkey(),
                &token::ID,
            );

        // Mint tokens
        let mint_ix = token::spl_token::instruction::mint_to(
            &token::ID,
            &mint.pubkey(),
            &contributor_ata,
            &payer.pubkey(),
            &[],
            contribution_amount,
        )
        .unwrap();

        rpc.create_and_send_transaction(&[create_ata_ix, mint_ix], &payer.pubkey(), &[&payer])
            .await
            .expect("Setup contributor should succeed");

        // Derive contributor account PDA
        let (contributor_account_pda, _) = Pubkey::find_program_address(
            &[
                b"contributor",
                fundraiser_pda.as_ref(),
                contributor.pubkey().as_ref(),
            ],
            &program_id,
        );

        let contribute_accounts = fundraiser::accounts::Contribute {
            contributor: contributor.pubkey(),
            mint_to_raise: mint.pubkey(),
            fundraiser: fundraiser_pda,
            contributor_account: contributor_account_pda,
            contributor_ata,
            vault: vault_pda,
            token_program: token::ID,
            system_program: solana_sdk::system_program::ID,
            light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            light_token_rent_sponsor: RENT_SPONSOR,
            light_token_cpi_authority: CPI_AUTHORITY_PDA,
            spl_interface_pda,
        };

        let contribute_data = fundraiser::instruction::Contribute {
            amount: contribution_amount,
        };

        let contribute_ix = Instruction {
            program_id,
            accounts: contribute_accounts.to_account_metas(None),
            data: contribute_data.data(),
        };

        rpc.create_and_send_transaction(&[contribute_ix], &payer.pubkey(), &[&payer, &contributor])
            .await
            .expect("contribute should succeed");

        total_contributed += contribution_amount;
        println!("Contributor {} contributed, total: {} tokens", i + 3, total_contributed / 1_000_000_000);
    }

    // Verify vault has reached target
    verify_light_token_balance(&mut rpc, vault_pda, amount_to_raise, "vault (target reached)").await;

    // === STEP 8: Check Contributions (maker claims funds) ===
    println!("\n=== Maker Claims Funds ===");

    // Create maker's SPL ATA (required for check_contributions - receives funds from Light vault)
    let create_maker_ata_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &maker.pubkey(),
            &mint.pubkey(),
            &token::ID,
        );

    rpc.create_and_send_transaction(&[create_maker_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Create maker ATA should succeed");

    let check_contributions_accounts = fundraiser::accounts::CheckContributions {
        fee_payer: maker.pubkey(),
        mint_to_raise: mint.pubkey(),
        fundraiser: fundraiser_pda,
        vault: vault_pda,
        maker_ata,
        token_program: token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda,
    };

    let check_contributions_data = fundraiser::instruction::CheckContributions {};

    let check_contributions_ix = Instruction {
        program_id,
        accounts: check_contributions_accounts.to_account_metas(None),
        data: check_contributions_data.data(),
    };

    rpc.create_and_send_transaction(&[check_contributions_ix], &payer.pubkey(), &[&payer, &maker])
        .await
        .expect("check_contributions should succeed");

    println!("Maker claimed funds!");

    // Verify vault is empty
    verify_light_token_balance(&mut rpc, vault_pda, 0, "vault (after claim)").await;

    // Verify maker received funds (check SPL ATA balance)
    let maker_ata_account = rpc
        .get_account(maker_ata)
        .await
        .unwrap()
        .expect("Maker ATA should exist");
    // SPL token accounts are 165 bytes
    let maker_token =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&maker_ata_account.data[..165]).unwrap();
    let maker_balance = u64::from(maker_token.amount);
    println!("Maker received {} tokens", maker_balance / 1_000_000_000);
    assert_eq!(maker_balance, amount_to_raise, "Maker should have received all raised funds");

    // Verify fundraiser account was closed
    let fundraiser_account = rpc.get_account(fundraiser_pda).await.unwrap();
    assert!(fundraiser_account.is_none(), "Fundraiser should be closed");

    println!("\n=== Fundraiser full flow test completed successfully! ===");
}

/// Test fundraiser initialization
#[tokio::test]
async fn test_initialize_fundraiser() {
    let program_id = fundraiser::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("fundraiser", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Setup program data for rent-free config
    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize rent-free config
    let (init_config_ix, _config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    // Create mint
    let mint = Keypair::new();
    let decimals = 9u8;

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(82)
        .await
        .unwrap();

    let create_mint_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        82,
        &token::ID,
    );
    let init_mint_ix = token::spl_token::instruction::initialize_mint(
        &token::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint_ix, init_mint_ix],
        &payer.pubkey(),
        &[&payer, &mint],
    )
    .await
    .expect("Create mint should succeed");

    // Create maker
    let maker = Keypair::new();
    rpc.airdrop_lamports(&maker.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Test valid initialization
    let amount_to_raise = 1_000_000_000_000u64; // 1000 tokens
    let duration = 30u16; // 30 days

    let (fundraiser_pda, _) = Pubkey::find_program_address(
        &[b"fundraiser", maker.pubkey().as_ref()],
        &program_id,
    );

    let (vault_pda, vault_bump) = Pubkey::find_program_address(
        &[fundraiser::VAULT_SEED, fundraiser_pda.as_ref()],
        &program_id,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(vault_pda)],
    )
    .await
    .unwrap();

    let initialize_accounts = fundraiser::accounts::Initialize {
        fee_payer: maker.pubkey(),
        mint_to_raise: mint.pubkey(),
        fundraiser: fundraiser_pda,
        vault: vault_pda,
        token_program: token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
    };

    let initialize_data = fundraiser::instruction::Initialize {
        params: fundraiser::instructions::InitializeParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            amount: amount_to_raise,
            duration,
            vault_bump,
        },
    };

    let initialize_ix = Instruction {
        program_id,
        accounts: [
            initialize_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: initialize_data.data(),
    };

    rpc.create_and_send_transaction(&[initialize_ix], &payer.pubkey(), &[&payer, &maker])
        .await
        .expect("initialize should succeed");

    // Verify state
    let fundraiser_account = rpc
        .get_account(fundraiser_pda)
        .await
        .unwrap()
        .expect("Fundraiser should exist");
    assert!(!fundraiser_account.data.is_empty());

    verify_light_token_balance(&mut rpc, vault_pda, 0, "vault").await;

    println!("=== Initialize fundraiser test completed successfully! ===");
}
