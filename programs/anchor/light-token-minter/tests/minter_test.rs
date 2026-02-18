//! Integration tests for Light Token mint creation using #[light_account(init, mint)] macro.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_account::derive_rent_sponsor_pda;
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{
    derive_token_ata, find_mint_address, LIGHT_TOKEN_CONFIG as COMPRESSIBLE_CONFIG_V1,
    LIGHT_TOKEN_RENT_SPONSOR as RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test creating a Light Token mint using the #[light_account(init, mint)] macro.
#[tokio::test]
async fn test_create_light_mint() {
    let program_id = light_token_minter::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("light_token_minter", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

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

    let authority = Keypair::new();

    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[
            light_token_minter::MINT_SIGNER_SEED,
            authority.pubkey().as_ref(),
        ],
        &program_id,
    );

    let (light_mint_pda, _) = find_mint_address(&mint_signer_pda);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    let accounts = light_token_minter::accounts::CreateMint {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        light_mint: light_mint_pda,
        compression_config: config_pda,
        light_token_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token::constants::LIGHT_TOKEN_CPI_AUTHORITY.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = light_token_minter::instruction::CreateMint {
        params: light_token_minter::CreateMintParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            decimals: 9,
            mint_signer_bump,
            token_name: "Test Token".to_string(),
            token_symbol: "TEST".to_string(),
            token_uri: "https://example.com/metadata.json".to_string(),
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateMint should succeed");

    let light_mint_account = rpc
        .get_account(light_mint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");

    assert!(
        !light_mint_account.data.is_empty(),
        "Mint account should have data"
    );

    assert!(
        light_mint_account.data.len() > 50,
        "Mint account should have sufficient data for a Light Token mint"
    );

}

/// Test basic mint signer PDA derivation without creating the mint.
#[tokio::test]
async fn test_mint_signer_derivation() {
    let program_id = light_token_minter::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("light_token_minter", program_id)]));
    config = config.with_light_protocol_events();

    let rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let (mint_signer_pda, bump) = Pubkey::find_program_address(
        &[
            light_token_minter::MINT_SIGNER_SEED,
            payer.pubkey().as_ref(),
        ],
        &program_id,
    );

    let (mint_pda, _) = find_mint_address(&mint_signer_pda);

    let (verify_signer, verify_bump) = Pubkey::find_program_address(
        &[
            light_token_minter::MINT_SIGNER_SEED,
            payer.pubkey().as_ref(),
        ],
        &program_id,
    );
    assert_eq!(mint_signer_pda, verify_signer);
    assert_eq!(bump, verify_bump);
}

/// Test the full flow: create Light Token mint, create token account, mint tokens.
#[tokio::test]
async fn test_mint_to() {
    let program_id = light_token_minter::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("light_token_minter", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

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

    let authority = Keypair::new();
    rpc.airdrop_lamports(&authority.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[
            light_token_minter::MINT_SIGNER_SEED,
            authority.pubkey().as_ref(),
        ],
        &program_id,
    );

    let (light_mint_pda, _) = find_mint_address(&mint_signer_pda);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    let create_mint_accounts = light_token_minter::accounts::CreateMint {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        light_mint: light_mint_pda,
        compression_config: config_pda,
        light_token_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token::constants::LIGHT_TOKEN_CPI_AUTHORITY.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let create_mint_data = light_token_minter::instruction::CreateMint {
        params: light_token_minter::CreateMintParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            decimals: 9,
            mint_signer_bump,
            token_name: "Test Token".to_string(),
            token_symbol: "TEST".to_string(),
            token_uri: "https://example.com/metadata.json".to_string(),
        },
    };

    let create_mint_ix = Instruction {
        program_id,
        accounts: [
            create_mint_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: create_mint_data.data(),
    };

    rpc.create_and_send_transaction(&[create_mint_ix], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateMint should succeed");

    let light_mint_account = rpc
        .get_account(light_mint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");
    assert!(
        !light_mint_account.data.is_empty(),
        "Mint account should have data"
    );

    let recipient = Keypair::new();
    rpc.airdrop_lamports(&recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let recipient_ata = derive_token_ata(&recipient.pubkey(), &light_mint_pda);

    let create_ata_ix = light_token::instruction::CreateAssociatedTokenAccount::new(
        payer.pubkey(),
        recipient.pubkey(),
        light_mint_pda,
    )
    .instruction()
    .unwrap();
    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Create recipient ATA");

    let mint_amount = 1_000_000_000u64;

    let mint_to_accounts = light_token_minter::accounts::MintTo {
        fee_payer: payer.pubkey(),
        mint_authority: authority.pubkey(),
        mint: light_mint_pda,
        recipient: recipient.pubkey(),
        destination: recipient_ata,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
        light_token_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: light_token::constants::LIGHT_TOKEN_CPI_AUTHORITY.into(),
    };

    let mint_to_data = light_token_minter::instruction::MintTo {
        params: light_token_minter::MintTokenParams {
            amount: mint_amount,
        },
    };

    let mint_to_ix = Instruction {
        program_id,
        accounts: mint_to_accounts.to_account_metas(None),
        data: mint_to_data.data(),
    };

    rpc.create_and_send_transaction(&[mint_to_ix], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("MintTo should succeed");

    let ata_account_after = rpc
        .get_account(recipient_ata)
        .await
        .unwrap()
        .expect("ATA should still exist");

    // Light Token accounts have SPL-compatible data in first 165 bytes
    use spl_token_2022::pod::PodAccount;
    if ata_account_after.data.len() >= 165 {
        let token_state =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ata_account_after.data[..165])
                .unwrap();
        let balance = u64::from(token_state.amount);
        assert_eq!(
            balance, mint_amount,
            "Token balance should match minted amount"
        );
    } else {
        panic!(
            "ATA data too short: {} bytes (expected >= 165)",
            ata_account_after.data.len()
        );
    }

}
