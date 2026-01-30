use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_token_anchor_create_mint::{accounts, instruction::CreateMint, ID};
use light_token::instruction::{
    config_pda, derive_mint_compressed_address, find_mint_address, rent_sponsor_pda,
    SystemAccounts, LIGHT_TOKEN_PROGRAM_ID, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
use anchor_lang::system_program;
use solana_sdk::{
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
};

#[tokio::test]
async fn test_create_mint() {
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("light_token_anchor_create_mint", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_seed = Keypair::new();
    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compression_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree.tree);

    let (mint_pda, bump) = find_mint_address(&mint_seed.pubkey());

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

    let system_accounts = SystemAccounts::default();

    // Call the anchor program to create mint
    let ix = Instruction {
        program_id: ID,
        accounts: accounts::CreateMintAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            mint_seed: mint_seed.pubkey(),
            authority: mint_authority,
            payer: payer.pubkey(),
            address_tree: address_tree.tree,
            output_queue,
            light_system_program: system_accounts.light_system_program,
            cpi_authority_pda: system_accounts.cpi_authority_pda,
            registered_program_pda: system_accounts.registered_program_pda,
            account_compression_authority: system_accounts.account_compression_authority,
            account_compression_program: system_accounts.account_compression_program,
            system_program: system_program::ID,
            compressible_config: config_pda(),
            mint: mint_pda,
            rent_sponsor: rent_sponsor_pda(),
        }
        .to_account_metas(Some(true)),
        data: CreateMint {
            decimals,
            address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
            compression_address: compression_address.into(),
            proof: rpc_result.proof.0.unwrap(),
            freeze_authority: None,
            bump,
            rent_payment: Some(DEFAULT_RENT_PAYMENT),
            write_top_up: Some(DEFAULT_WRITE_TOP_UP),
            metadata: None,
        }
        .data(),
    };

    let sig = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &mint_seed])
        .await
        .unwrap();

    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;

    assert!(compressed_account.is_some(), "Light-mint should exist");
    println!("Tx: {}", sig);
}
