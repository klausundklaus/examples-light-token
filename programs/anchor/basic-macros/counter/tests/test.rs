use anchor_lang::{InstructionData, ToAccountMetas};
use counter::{CreateCounterParams, COUNTER_SEED, ID};
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_token::instruction::RENT_SPONSOR;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_create_counter() {
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("counter", ID)]),
    )
    .with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &ID);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &ID,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let owner = Keypair::new();

    let (counter_pda, _bump) =
        Pubkey::find_program_address(&[COUNTER_SEED, owner.pubkey().as_ref()], &ID);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &ID,
        vec![CreateAccountsProofInput::pda(counter_pda)],
    )
    .await
    .unwrap();

    let accounts = counter::accounts::CreateCounter {
        fee_payer: payer.pubkey(),
        owner: owner.pubkey(),
        compression_config: config_pda,
        counter: counter_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = counter::instruction::CreateCounter {
        params: CreateCounterParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            count: 42,
        },
    };

    let ix = Instruction {
        program_id: ID,
        accounts: [accounts.to_account_metas(None), proof_result.remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let sig = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
