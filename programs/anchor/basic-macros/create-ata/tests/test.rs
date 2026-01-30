//! Integration test for Light Protocol associated token account creation using the macro.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{get_create_accounts_proof, InitializeRentFreeConfig};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Setup helper: Creates a compressed mint directly using the ctoken SDK.
/// Returns (mint_pda, mint_seed_keypair)
async fn setup_create_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
) -> (Pubkey, Keypair) {
    use light_token::instruction::{CreateMint, CreateMintParams, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = light_token::instruction::find_mint_address(&mint_seed.pubkey());

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
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
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: DEFAULT_RENT_PAYMENT,
        write_top_up: DEFAULT_WRITE_TOP_UP,
    };

    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    (mint, mint_seed)
}

/// Test creating a Light Protocol associated token account using the macro.
#[tokio::test]
async fn test_create_associated_token_account() {
    use light_token_macro_create_associated_token_account::CreateAssociatedTokenAccountParams;

    let program_id = light_token_macro_create_associated_token_account::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("light_token_macro_create_associated_token_account", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

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

    // Setup mint first
    let (mint, _mint_seed) = setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(), // mint_authority
        9,              // decimals
    )
    .await;

    // The associated token account owner will be the payer
    let associated_token_account_owner = payer.pubkey();

    // Derive the associated token account address using Light Token SDK's derivation
    let (associated_token_account, associated_token_account_bump) = light_token::instruction::derive_token_ata(&associated_token_account_owner, &mint);

    // Get proof (no PDA accounts for associated token account-only instruction)
    let proof_result = get_create_accounts_proof(&rpc, &program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = light_token_macro_create_associated_token_account::accounts::CreateAssociatedTokenAccount {
        fee_payer: payer.pubkey(),
        associated_token_account_mint: mint,
        associated_token_account_owner,
        associated_token_account,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = light_token_macro_create_associated_token_account::instruction::CreateAssociatedTokenAccount {
        params: CreateAssociatedTokenAccountParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            associated_token_account_bump,
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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateAssociatedTokenAccount instruction should succeed");

    // Verify associated token account exists on-chain
    let associated_token_account_data = rpc
        .get_account(associated_token_account)
        .await
        .unwrap()
        .expect("Associated token account should exist on-chain");

    // Parse and verify token data
    use light_token_interface::state::Token;
    let token: Token = borsh::BorshDeserialize::deserialize(&mut &associated_token_account_data.data[..])
        .expect("Failed to deserialize Token");

    // Verify owner
    assert_eq!(token.owner, associated_token_account_owner.to_bytes(), "Associated token account owner should match");

    // Verify mint
    assert_eq!(token.mint, mint.to_bytes(), "Associated token account mint should match");

    // Verify initial amount is 0
    assert_eq!(token.amount, 0, "Associated token account amount should be 0 initially");
}
