//! Shared test utilities for programs-sdk examples.

use light_client::indexer::AddressWithTree;
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_token::instruction::{
    CreateAssociatedTokenAccount, CreateMint, CreateMintParams, MintTo,
    derive_mint_compressed_address, derive_token_ata, find_mint_address,
    DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Test environment with RPC, payer, and mint details.
pub struct TestEnv {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint_pda: Pubkey,
    pub associated_token_account: Pubkey,
}

/// Test environment with freeze authority enabled on mint.
pub struct TestEnvWithFreeze {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint_pda: Pubkey,
    pub associated_token_account: Pubkey,
    pub freeze_authority: Pubkey,
}

/// Sets up a test environment with an initialized Light token mint and associated token account.
pub async fn setup_test_env(program_name: &'static str, program_id: Pubkey) -> TestEnv {
    let config = ProgramTestConfig::new_v2(true, Some(vec![(program_name, program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let (mint_pda, _mint_seed) = create_mint(&mut rpc, &payer, None).await;
    let associated_token_account = create_associated_token_account(&mut rpc, &payer, &mint_pda).await;

    TestEnv {
        rpc,
        payer,
        mint_pda,
        associated_token_account,
    }
}

/// Sets up a test environment with freeze authority enabled on the mint.
pub async fn setup_test_env_with_freeze(
    program_name: &'static str,
    program_id: Pubkey,
) -> TestEnvWithFreeze {
    let config = ProgramTestConfig::new_v2(true, Some(vec![(program_name, program_id)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let freeze_authority = payer.pubkey();

    let (mint_pda, _mint_seed) = create_mint(&mut rpc, &payer, Some(freeze_authority)).await;
    let associated_token_account = create_associated_token_account(&mut rpc, &payer, &mint_pda).await;

    TestEnvWithFreeze {
        rpc,
        payer,
        mint_pda,
        associated_token_account,
        freeze_authority,
    }
}

/// Creates a Light-mint with optional freeze authority. Returns its PDA and seed keypair.
pub async fn create_mint(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    freeze_authority: Option<Pubkey>,
) -> (Pubkey, Keypair) {
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
        freeze_authority,
        extensions: None,
        rent_payment: DEFAULT_RENT_PAYMENT,
        write_top_up: DEFAULT_WRITE_TOP_UP,
    };

    let create_ix = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    )
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    (mint_pda, mint_seed)
}

/// Creates a Light associated token account for the payer.
pub async fn create_associated_token_account(rpc: &mut LightProgramTest, payer: &Keypair, mint_pda: &Pubkey) -> Pubkey {
    create_associated_token_account_for_owner(rpc, payer, &payer.pubkey(), mint_pda).await
}

/// Creates a Light associated token account for a specific owner.
pub async fn create_associated_token_account_for_owner(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    owner: &Pubkey,
    mint_pda: &Pubkey,
) -> Pubkey {
    let associated_token_account = derive_token_ata(owner, mint_pda);
    let create_associated_token_account_ix = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, *mint_pda)
        .instruction()
        .unwrap();

    rpc.create_and_send_transaction(&[create_associated_token_account_ix], &payer.pubkey(), &[payer])
        .await
        .unwrap();

    associated_token_account
}

/// Mints tokens to the specified associated token account.
pub async fn mint_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mint_pda: Pubkey,
    associated_token_account: Pubkey,
    amount: u64,
) {
    let mint_to_ix = MintTo {
        mint: mint_pda,
        destination: associated_token_account,
        amount,
        authority: payer.pubkey(),
        fee_payer: payer.pubkey(),
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[mint_to_ix], &payer.pubkey(), &[payer])
        .await
        .unwrap();
}
