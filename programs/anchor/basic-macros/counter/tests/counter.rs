//! Light-PDA lifecycle test: create → increment → close.
//!
//! Only `create_counter` uses Light-specific attributes (`#[light_account(init)]`,
//! `LightAccounts`, `CreateAccountsProof`). The `increment` and `close_counter`
//! instructions are standard Anchor.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const PROGRAM_ID: Pubkey = counter::ID;

/// Derives the rent sponsor PDA for the counter program.
fn rent_sponsor_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"rent_sponsor"], &PROGRAM_ID).0
}

/// Setup: create test RPC and initialize rent-free config for the counter program.
async fn setup() -> (LightProgramTest, Keypair, Pubkey) {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("counter", PROGRAM_ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &PROGRAM_ID);

    // Register this program for rent-free accounts. One-time setup per program.
    // The resulting config PDA is passed to create_counter as `compression_config`.
    let (init_config_ix, compression_config) = InitializeRentFreeConfig::new(
        &PROGRAM_ID,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor_pda(),
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("initialize rent-free config");

    // Fund the rent sponsor PDA. In production, the protocol funds this automatically.
    let fund_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &rent_sponsor_pda(),
        10_000_000,
    );
    rpc.create_and_send_transaction(&[fund_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("fund rent sponsor");

    (rpc, payer, compression_config)
}

#[tokio::test]
async fn test_counter_lifecycle() {
    let (mut rpc, payer, compression_config) = setup().await;

    // ── Create ───────────────────────────────────────────────────────────
    let (counter_pda, _) = Pubkey::find_program_address(
        &[counter::COUNTER_SEED, payer.pubkey().as_ref()],
        &PROGRAM_ID,
    );

    let proof_result = get_create_accounts_proof(
        &rpc,
        &PROGRAM_ID,
        vec![CreateAccountsProofInput::pda(counter_pda)],
    )
    .await
    .unwrap();

    let create_accounts = counter::accounts::CreateCounter {
        fee_payer: payer.pubkey(),
        owner: payer.pubkey(),
        compression_config,
        pda_rent_sponsor: rent_sponsor_pda(),
        counter: counter_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let create_data = counter::instruction::CreateCounter {
        params: counter::CreateCounterParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            count: 0,
        },
    };

    let create_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: [
            create_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: create_data.data(),
    };

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("create_counter");

    // Verify initial state.
    let account = rpc.get_account(counter_pda).await.unwrap().unwrap();
    let ctr: counter::Counter =
        anchor_lang::AccountDeserialize::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(ctr.count, 0);
    assert_eq!(ctr.owner, payer.pubkey());

    // ── Increment (standard Anchor) ────────────────────────────
    let inc_accounts = counter::accounts::Increment {
        owner: payer.pubkey(),
        counter: counter_pda,
    };
    let inc_data = counter::instruction::Increment {};
    let inc_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: inc_accounts.to_account_metas(None),
        data: inc_data.data(),
    };

    rpc.create_and_send_transaction(&[inc_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("increment");

    let account = rpc.get_account(counter_pda).await.unwrap().unwrap();
    let ctr: counter::Counter =
        anchor_lang::AccountDeserialize::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(ctr.count, 1);

    // ── Close (standard Anchor) ────────────────────────────────
    let close_accounts = counter::accounts::CloseCounter {
        fee_payer: payer.pubkey(),
        owner: payer.pubkey(),
        counter: counter_pda,
    };
    let close_data = counter::instruction::CloseCounter {};
    let close_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: close_accounts.to_account_metas(None),
        data: close_data.data(),
    };

    rpc.create_and_send_transaction(&[close_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("close_counter");

    // Account should no longer exist.
    let account = rpc.get_account(counter_pda).await.unwrap();
    assert!(account.is_none(), "counter should be closed");
}