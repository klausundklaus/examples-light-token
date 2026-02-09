#![allow(dead_code)]

//! Shared test utilities for the pinocchio-swap program.

use light_account::derive_rent_sponsor_pda;
use light_client::interface::InitializeRentFreeConfig;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Shared test environment with initialized compression config.
pub struct TestEnv {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub program_id: Pubkey,
    pub config_pda: Pubkey,
    pub rent_sponsor: Pubkey,
}

/// Sets up a test environment with program, config, and rent sponsor initialized.
pub async fn setup_test_env() -> TestEnv {
    let program_id = Pubkey::new_from_array(pinocchio_swap::ID);
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("pinocchio_swap", program_id)]));
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

    // Fund the rent sponsor so it can pay for rent reimbursements
    rpc.airdrop_lamports(&rent_sponsor, 1_000_000_000)
        .await
        .expect("Airdrop to rent sponsor should succeed");

    TestEnv {
        rpc,
        payer,
        program_id,
        config_pda,
        rent_sponsor,
    }
}

/// PDAs for the swap pool.
pub struct PoolPdas {
    pub pool_state: Pubkey,
    pub pool_bump: u8,
    pub pool_authority: Pubkey,
    pub pool_authority_bump: u8,
    pub vault_a: Pubkey,
    pub vault_a_bump: u8,
    pub vault_b: Pubkey,
    pub vault_b_bump: u8,
}

/// Token setup for the pool.
pub struct TokenSetup {
    pub mint_a: Pubkey,
    pub mint_a_signer: Pubkey,
    pub mint_a_signer_bump: u8,
    pub mint_b: Pubkey,
    pub mint_b_signer: Pubkey,
    pub mint_b_signer_bump: u8,
    pub user_token_a: Pubkey,
    pub user_token_b: Pubkey,
}

/// Derive the global pool authority PDA (same for all pools).
pub fn derive_pool_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    use pinocchio_swap::constants::POOL_AUTHORITY_SEED;
    Pubkey::find_program_address(&[POOL_AUTHORITY_SEED], program_id)
}

/// Derive all pool PDAs from token mints and authority.
pub fn derive_pool_pdas(
    program_id: &Pubkey,
    authority: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> (PoolPdas, TokenSetup) {
    use pinocchio_swap::constants::*;

    // Mint signers
    let (mint_a_signer, mint_a_signer_bump) =
        Pubkey::find_program_address(&[MINT_A_SEED, authority.as_ref()], program_id);
    let (mint_b_signer, mint_b_signer_bump) =
        Pubkey::find_program_address(&[MINT_B_SEED, authority.as_ref()], program_id);

    // Pool state
    let (pool_state, pool_bump) =
        Pubkey::find_program_address(&[POOL_SEED, mint_a.as_ref(), mint_b.as_ref()], program_id);

    // Global pool authority (same for all pools)
    let (pool_authority, pool_authority_bump) = derive_pool_authority(program_id);

    // Vaults
    let (vault_a, vault_a_bump) = Pubkey::find_program_address(
        &[POOL_VAULT_SEED, pool_state.as_ref(), mint_a.as_ref()],
        program_id,
    );
    let (vault_b, vault_b_bump) = Pubkey::find_program_address(
        &[POOL_VAULT_SEED, pool_state.as_ref(), mint_b.as_ref()],
        program_id,
    );

    let pool_pdas = PoolPdas {
        pool_state,
        pool_bump,
        pool_authority,
        pool_authority_bump,
        vault_a,
        vault_a_bump,
        vault_b,
        vault_b_bump,
    };

    let token_setup = TokenSetup {
        mint_a: *mint_a,
        mint_a_signer,
        mint_a_signer_bump,
        mint_b: *mint_b,
        mint_b_signer,
        mint_b_signer_bump,
        user_token_a: Pubkey::default(), // Will be set later
        user_token_b: Pubkey::default(), // Will be set later
    };

    (pool_pdas, token_setup)
}

/// Asserts an account exists on-chain.
pub async fn assert_onchain_exists(rpc: &mut LightProgramTest, pda: &Pubkey, name: &str) {
    assert!(
        rpc.get_account(*pda).await.unwrap().is_some(),
        "{} account ({}) should exist on-chain",
        name,
        pda
    );
}

/// Asserts an account is closed on-chain.
pub async fn assert_onchain_closed(rpc: &mut LightProgramTest, pda: &Pubkey, name: &str) {
    let acc = rpc.get_account(*pda).await.unwrap();
    assert!(
        acc.is_none() || acc.unwrap().lamports == 0,
        "{} account ({}) should be closed",
        name,
        pda
    );
}

/// Build instruction data: discriminator + borsh-serialized params.
pub fn build_instruction_data<T: borsh::BorshSerialize>(disc: &[u8; 8], params: &T) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(disc);
    borsh::BorshSerialize::serialize(params, &mut data).unwrap();
    data
}

/// Asserts all pool-related accounts exist on-chain.
pub async fn assert_all_pool_accounts_exist(
    rpc: &mut LightProgramTest,
    pdas: &PoolPdas,
    tokens: &TokenSetup,
) {
    assert_onchain_exists(rpc, &pdas.pool_state, "PoolState").await;
    assert_onchain_exists(rpc, &tokens.mint_a, "MintA").await;
    assert_onchain_exists(rpc, &tokens.mint_b, "MintB").await;
    assert_onchain_exists(rpc, &pdas.vault_a, "VaultA").await;
    assert_onchain_exists(rpc, &pdas.vault_b, "VaultB").await;
}

/// Asserts all pool-related accounts are closed on-chain.
pub async fn assert_all_pool_accounts_closed(
    rpc: &mut LightProgramTest,
    pdas: &PoolPdas,
    tokens: &TokenSetup,
) {
    assert_onchain_closed(rpc, &pdas.pool_state, "PoolState").await;
    assert_onchain_closed(rpc, &tokens.mint_a, "MintA").await;
    assert_onchain_closed(rpc, &tokens.mint_b, "MintB").await;
    assert_onchain_closed(rpc, &pdas.vault_a, "VaultA").await;
    assert_onchain_closed(rpc, &pdas.vault_b, "VaultB").await;
}
