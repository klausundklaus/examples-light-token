//! Integration tests for the escrow program with Light Token accounts.
//!
//! This test demonstrates a full escrow flow using:
//! - Light mints (token A and token B) - created via light_token::CreateMint
//! - Light Token accounts for maker and taker - created via CreateAssociatedTokenAccount
//! - Make offer: maker deposits token A into vault, creates offer
//! - Take offer: taker sends token B to maker, receives token A from vault

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::indexer::AddressWithTree;
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
use light_token::constants::CPI_AUTHORITY_PDA;
use light_token::instruction::{
    derive_mint_compressed_address, derive_token_ata, find_mint_address,
    CreateAssociatedTokenAccount, CreateMint, CreateMintParams, MintTo, LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Create a Light mint using the light_token SDK.
async fn create_light_mint<R: Rpc + Indexer>(
    rpc: &mut R,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
) -> (Pubkey, Keypair) {
    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compression_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree.tree);
    let (mint, bump) = find_mint_address(&mint_seed.pubkey());

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let params = CreateMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority: *mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };

    let instruction = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    )
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    (mint, mint_seed)
}

/// Create a Light Token ATA.
async fn create_light_token_ata<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let (ata, _) = derive_token_ata(owner, mint);

    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, *mint)
        .instruction()
        .unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
        .unwrap();

    ata
}

/// Mint tokens to a Light Token account.
async fn mint_light_tokens<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) {
    let instruction = MintTo {
        fee_payer: Some(payer.pubkey()),
        mint: *mint,
        destination: *destination,
        amount,
        authority: mint_authority.pubkey(),
        max_top_up: None,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, mint_authority])
        .await
        .unwrap();
}

/// Verify Light Token account balance.
async fn verify_light_token_balance<R: Rpc>(
    rpc: &mut R,
    account: Pubkey,
    expected: u64,
    name: &str,
) {
    use spl_token_2022::pod::PodAccount;

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

/// Test the full escrow flow: make_offer and take_offer with Light Token accounts.
#[tokio::test]
async fn test_escrow_full_flow() {
    let program_id = escrow::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("escrow", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Setup program data for rent-free config
    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize rent-free config
    let rent_sponsor = escrow::program_rent_sponsor();
    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    // Fund the rent sponsor PDA so it can pay for rent reimbursements
    rpc.airdrop_lamports(&rent_sponsor, 1_000_000_000)
        .await
        .expect("Airdrop to rent sponsor should succeed");

    println!("Rent-free config initialized at: {:?}", config_pda);

    // Create maker and taker keypairs (also serve as mint authorities for simplicity)
    let maker = Keypair::new();
    let taker = Keypair::new();

    // Airdrop lamports to maker and taker
    rpc.airdrop_lamports(&maker.pubkey(), 2_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&taker.pubkey(), 2_000_000_000)
        .await
        .unwrap();

    // Create Light mints for token A and token B
    // Maker is mint authority for token A, taker is mint authority for token B
    println!("\nCreating Light mints...");
    let (mint_a, _mint_seed_a) = create_light_mint(&mut rpc, &payer, &payer.pubkey(), 9).await;
    let (mint_b, _mint_seed_b) = create_light_mint(&mut rpc, &payer, &payer.pubkey(), 9).await;

    println!("Created mint A: {:?}", mint_a);
    println!("Created mint B: {:?}", mint_b);

    // Create Light Token ATAs for maker and taker
    println!("\nCreating Light Token ATAs...");
    let maker_ata_a = create_light_token_ata(&mut rpc, &payer, &mint_a, &maker.pubkey()).await;
    let maker_ata_b = create_light_token_ata(&mut rpc, &payer, &mint_b, &maker.pubkey()).await;
    let taker_ata_a = create_light_token_ata(&mut rpc, &payer, &mint_a, &taker.pubkey()).await;
    let taker_ata_b = create_light_token_ata(&mut rpc, &payer, &mint_b, &taker.pubkey()).await;

    println!("maker_ata_a: {:?}", maker_ata_a);
    println!("maker_ata_b: {:?}", maker_ata_b);
    println!("taker_ata_a: {:?}", taker_ata_a);
    println!("taker_ata_b: {:?}", taker_ata_b);

    // Mint tokens
    let token_a_amount = 1_000_000_000u64; // 1 token with 9 decimals
    let token_b_amount = 500_000_000u64; // 0.5 tokens

    println!("\nMinting tokens...");
    mint_light_tokens(
        &mut rpc,
        &payer,
        &mint_a,
        &maker_ata_a,
        &payer,
        token_a_amount,
    )
    .await;
    println!("Minted {} token A to maker", token_a_amount);

    mint_light_tokens(
        &mut rpc,
        &payer,
        &mint_b,
        &taker_ata_b,
        &payer,
        token_b_amount,
    )
    .await;
    println!("Minted {} token B to taker", token_b_amount);

    // Verify balances before escrow
    println!("\nVerifying initial balances...");
    verify_light_token_balance(&mut rpc, maker_ata_a, token_a_amount, "maker_ata_a").await;
    verify_light_token_balance(&mut rpc, taker_ata_b, token_b_amount, "taker_ata_b").await;
    verify_light_token_balance(&mut rpc, maker_ata_b, 0, "maker_ata_b").await;
    verify_light_token_balance(&mut rpc, taker_ata_a, 0, "taker_ata_a").await;

    // === MAKE OFFER ===
    println!("\n=== Executing make_offer ===");

    let offer_id = 1u64;
    let token_a_offered_amount = token_a_amount;
    let token_b_wanted_amount = token_b_amount;

    // Derive offer PDA
    let (offer_pda, _offer_bump) = Pubkey::find_program_address(
        &[
            escrow::OFFER_SEED,
            maker.pubkey().as_ref(),
            offer_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    // Derive vault PDA
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[escrow::VAULT_SEED, offer_pda.as_ref()], &program_id);

    // Derive authority PDA
    let (authority_pda, _authority_bump) =
        Pubkey::find_program_address(&[escrow::AUTH_SEED.as_bytes()], &program_id);

    println!("Offer PDA: {:?}", offer_pda);
    println!("Vault PDA: {:?}", vault_pda);
    println!("Authority PDA: {:?}", authority_pda);

    // Get proof for creating the offer account
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(offer_pda)],
    )
    .await
    .unwrap();

    // Build make_offer instruction
    let make_offer_accounts = escrow::accounts::MakeOffer {
        fee_payer: maker.pubkey(),
        authority: authority_pda,
        compression_config: config_pda,
        token_mint_a: mint_a,
        token_mint_b: mint_b,
        maker_token_account_a: maker_ata_a,
        offer: offer_pda,
        vault: vault_pda,
        token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        system_program: solana_sdk::system_program::ID,
        pda_rent_sponsor: rent_sponsor,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
    };

    let make_offer_data = escrow::instruction::MakeOffer {
        params: escrow::MakeOfferParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            id: offer_id,
            token_a_offered_amount,
            token_b_wanted_amount,
            vault_bump,
        },
    };

    let make_offer_ix = Instruction {
        program_id,
        accounts: [
            make_offer_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: make_offer_data.data(),
    };

    rpc.create_and_send_transaction(&[make_offer_ix], &maker.pubkey(), &[&maker])
        .await
        .expect("make_offer should succeed");

    println!("make_offer executed successfully!");

    // Verify offer account was created
    let offer_account = rpc
        .get_account(offer_pda)
        .await
        .unwrap()
        .expect("Offer account should exist");
    assert!(
        !offer_account.data.is_empty(),
        "Offer account should have data"
    );
    println!(
        "Offer account verified, size: {} bytes",
        offer_account.data.len()
    );

    // === TAKE OFFER ===
    println!("\n=== Executing take_offer ===");

    // Build take_offer instruction
    let take_offer_accounts = escrow::accounts::TakeOffer {
        taker: taker.pubkey(),
        maker: maker.pubkey(),
        authority: authority_pda,
        token_mint_a: mint_a,
        token_mint_b: mint_b,
        taker_token_account_a: taker_ata_a,
        taker_token_account_b: taker_ata_b,
        maker_token_account_b: maker_ata_b,
        offer: offer_pda,
        vault: vault_pda,
        token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        system_program: solana_sdk::system_program::ID,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
    };

    let take_offer_data = escrow::instruction::TakeOffer {};

    let take_offer_ix = Instruction {
        program_id,
        accounts: take_offer_accounts.to_account_metas(None),
        data: take_offer_data.data(),
    };

    rpc.create_and_send_transaction(&[take_offer_ix], &taker.pubkey(), &[&taker])
        .await
        .expect("take_offer should succeed");

    println!("take_offer executed successfully!");

    // === VERIFY FINAL BALANCES ===
    println!("\n=== Verifying final balances ===");

    // After escrow:
    // - Maker should have 0 token A, token_b_wanted_amount token B
    // - Taker should have token_a_offered_amount token A, 0 token B
    // - Vault should be empty

    verify_light_token_balance(&mut rpc, maker_ata_a, 0, "maker_ata_a (should be 0)").await;
    verify_light_token_balance(
        &mut rpc,
        maker_ata_b,
        token_b_wanted_amount,
        "maker_ata_b (received)",
    )
    .await;
    verify_light_token_balance(
        &mut rpc,
        taker_ata_a,
        token_a_offered_amount,
        "taker_ata_a (received)",
    )
    .await;
    verify_light_token_balance(&mut rpc, taker_ata_b, 0, "taker_ata_b (should be 0)").await;

    // Verify offer account was closed (rent returned to maker)
    let offer_after = rpc.get_account(offer_pda).await.unwrap();
    assert!(
        offer_after.is_none(),
        "Offer account should be closed after take_offer"
    );
    println!("Offer account closed successfully");

    println!("\n=== Escrow test completed successfully! ===");
}

/// Test that verifies offer PDA derivation and basic setup
#[tokio::test]
async fn test_escrow_setup() {
    let program_id = escrow::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("escrow", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize rent-free config
    let rent_sponsor = escrow::program_rent_sponsor();
    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    // Fund the rent sponsor PDA
    rpc.airdrop_lamports(&rent_sponsor, 1_000_000_000)
        .await
        .expect("Airdrop to rent sponsor should succeed");

    // Verify PDA derivations
    let offer_id = 1u64;
    let (offer_pda, _) = Pubkey::find_program_address(
        &[
            escrow::OFFER_SEED,
            payer.pubkey().as_ref(),
            offer_id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );

    let (vault_pda, _) =
        Pubkey::find_program_address(&[escrow::VAULT_SEED, offer_pda.as_ref()], &program_id);

    let (authority_pda, _) =
        Pubkey::find_program_address(&[escrow::AUTH_SEED.as_bytes()], &program_id);

    println!("Escrow setup test completed successfully");
    println!("Program ID: {:?}", program_id);
    println!("Config PDA: {:?}", config_pda);
    println!("Offer PDA: {:?}", offer_pda);
    println!("Vault PDA: {:?}", vault_pda);
    println!("Authority PDA: {:?}", authority_pda);

    // Get proof for creating accounts to verify proof system works
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(offer_pda)],
    )
    .await
    .unwrap();

    assert!(
        proof_result.create_accounts_proof.proof.0.is_some()
            || !proof_result.remaining_accounts.is_empty(),
        "Should have proof or remaining accounts"
    );
    println!("Proof generation verified");
}
