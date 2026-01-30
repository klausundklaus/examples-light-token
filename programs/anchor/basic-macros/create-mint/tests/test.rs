use anchor_lang::{prelude::borsh, InstructionData, ToAccountMetas};
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{find_mint_address, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use light_token_macro_create_mint::MINT_SIGNER_SEED;

async fn setup() -> (light_program_test::LightProgramTest, Keypair, Pubkey, Pubkey) {
    let program_id = light_token_macro_create_mint::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("light_token_macro_create_mint", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

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

    (rpc, payer, config_pda, program_id)
}

#[tokio::test]
async fn test_create_mint() {
    use light_token_macro_create_mint::CreateMintParams;

    let (mut rpc, payer, config_pda, program_id) = setup().await;

    let authority = Keypair::new();
    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_pda, _) = find_mint_address(&mint_signer_pda);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    let accounts = light_token_macro_create_mint::accounts::CreateMint {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        mint: mint_pda,
        compression_config: config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = light_token_macro_create_mint::instruction::CreateMint {
        params: CreateMintParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_bump,
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

    let mint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");

    use light_token_interface::state::Mint;
    let mint: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_account.data[..])
        .expect("Failed to deserialize Mint");

    assert_eq!(mint.base.decimals, 9);
    assert_eq!(
        mint.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
    );
}

#[tokio::test]
async fn test_create_mint_with_metadata() {
    use light_token_macro_create_mint::CreateMintWithMetadataParams;

    let (mut rpc, payer, config_pda, program_id) = setup().await;

    let authority = Keypair::new();
    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_pda, _) = find_mint_address(&mint_signer_pda);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    let accounts = light_token_macro_create_mint::accounts::CreateMintWithMetadata {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        mint: mint_pda,
        compression_config: config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = light_token_macro_create_mint::instruction::CreateMintWithMetadata {
        params: CreateMintWithMetadataParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_bump,
            name: b"Test Token".to_vec(),
            symbol: b"TST".to_vec(),
            uri: b"https://example.com/metadata.json".to_vec(),
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
        .expect("CreateMintWithMetadata should succeed");

    let mint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");

    use light_token_interface::state::Mint;
    let mint: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_account.data[..])
        .expect("Failed to deserialize Mint");

    assert_eq!(mint.base.decimals, 9);
    assert_eq!(
        mint.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
    );
}
