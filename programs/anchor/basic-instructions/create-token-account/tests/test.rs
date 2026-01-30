use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::indexer::AddressWithTree;
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_token_anchor_create_token_account::{accounts, instruction::CreateTokenAccount, ID};
use light_token::instruction::{
    CreateMint, CreateMintParams, config_pda, derive_mint_compressed_address,
    find_mint_address, rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID,
    DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
use anchor_lang::system_program;
use solana_sdk::{
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
};

#[tokio::test]
async fn test_create_token_account() {
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("light_token_anchor_create_token_account", ID)]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a mint first
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
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint: mint_pda,
        bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: DEFAULT_RENT_PAYMENT,
        write_top_up: DEFAULT_WRITE_TOP_UP,
    };

    let create_mint_ix = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    )
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[create_mint_ix], &payer.pubkey(), &[&payer, &mint_seed])
        .await
        .unwrap();

    // You can use light, spl, t22 mints to create a light token account.
    // Create a token account
    let token_account = Keypair::new();
    let owner = payer.pubkey();
    let compressible_config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let ix = Instruction {
        program_id: ID,
        accounts: accounts::CreateTokenAccountAccounts {
            payer: payer.pubkey(),
            account: token_account.pubkey(),
            mint: mint_pda,
            compressible_config,
            rent_sponsor,
            system_program: system_program::ID,
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        }
        .to_account_metas(Some(true)),
        data: CreateTokenAccount { owner }.data(),
    };

    let sig = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &token_account])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
