#![allow(dead_code)]

//! Shared test utilities for the pinocchio-counter program.

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
    let program_id = Pubkey::new_from_array(pinocchio_counter::ID);
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("pinocchio_counter", program_id)]))
            .with_light_protocol_events();

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

/// Derive the counter PDA.
pub fn derive_counter_pda(program_id: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[pinocchio_counter::COUNTER_SEED, owner.as_ref()],
        program_id,
    )
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

/// Build instruction data with just a discriminator (no params).
pub fn build_instruction_data_no_params(disc: &[u8; 8]) -> Vec<u8> {
    disc.to_vec()
}
