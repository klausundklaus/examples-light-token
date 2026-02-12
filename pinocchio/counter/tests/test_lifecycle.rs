//! Full lifecycle test for pinocchio-counter program.
//!
//! Tests the complete lifecycle:
//! 1. Setup - Initialize compression config
//! 2. Create - Create counter PDA with Light Protocol registration
//! 3. Increment - Mutate counter on-chain
//! 4. Advance Epoch - warp_slot_forward to trigger auto-compression
//! 5. Verify Compressed - Counter should be closed on-chain
//! 6. Decompress - Use CounterSdk + create_load_instructions to decompress
//! 7. Verify Restored - Counter exists again, state preserved
//! 8. Increment Again - Verify counter works after decompression

mod sdk;
mod shared;

use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterfaceExt,
    CreateAccountsProofInput, LightProgramInterface,
};
use light_program_test::program_test::LightProgramTest;
use light_program_test::{program_test::TestRpc, Rpc};
use pinocchio_counter::{
    create_counter::CreateCounterParams, discriminators, CounterState, LightAccountVariant,
};
use sdk::{CounterInstruction, CounterSdk};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Slots per epoch for compression timing (matches light-compressible).
const SLOTS_PER_EPOCH: u64 = 13500;

/// Build the create_counter instruction.
fn build_create_counter_instruction(
    program_id: Pubkey,
    payer: Pubkey,
    owner: Pubkey,
    counter_pda: Pubkey,
    counter_bump: u8,
    config_pda: Pubkey,
    proof_result: &light_client::interface::CreateAccountsProofResult,
) -> Instruction {
    let params = CreateCounterParams {
        create_accounts_proof: proof_result.create_accounts_proof.clone(),
        counter_bump,
    };

    // Account order per create_counter/accounts.rs:
    // [0] payer (signer, writable)
    // [1] owner (read-only)
    // [2] counter (writable)
    // [3] compressible_config (read-only)
    // [4] system_program (read-only)
    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(owner, false),
        AccountMeta::new(counter_pda, false),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts.clone()].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_COUNTER, &params),
    }
}

/// Build the increment instruction.
fn build_increment_instruction(
    program_id: Pubkey,
    owner: Pubkey,
    counter_pda: Pubkey,
) -> Instruction {
    // Account order per increment/accounts.rs:
    // [0] owner (signer)
    // [1] counter (writable)
    let accounts = vec![
        AccountMeta::new_readonly(owner, true),
        AccountMeta::new(counter_pda, false),
    ];

    Instruction {
        program_id,
        accounts,
        data: shared::build_instruction_data_no_params(&discriminators::INCREMENT),
    }
}

#[tokio::test]
async fn test_full_lifecycle() {
    // ==================== PHASE 1: Setup Environment ====================
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let owner = payer.pubkey();

    // ==================== PHASE 2: Derive Counter PDA ====================
    let (counter_pda, counter_bump) = shared::derive_counter_pda(&program_id, &owner);
    println!("Counter PDA: {}", counter_pda);

    // ==================== PHASE 3: Get Proof and Create Counter ====================
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(counter_pda)],
    )
    .await
    .expect("get_create_accounts_proof should succeed");

    let create_ix = build_create_counter_instruction(
        program_id,
        payer.pubkey(),
        owner,
        counter_pda,
        counter_bump,
        env.config_pda,
        &proof_result,
    );

    println!("Creating counter...");
    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("create_counter should succeed");

    // ==================== PHASE 4: Verify Counter Exists ====================
    shared::assert_onchain_exists(&mut rpc, &counter_pda, "Counter").await;

    // Verify initial state
    let account = rpc
        .get_account(counter_pda)
        .await
        .unwrap()
        .expect("Counter should exist");
    let counter: CounterState =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
    assert_eq!(counter.count, 0, "Initial count should be 0");
    assert_eq!(
        counter.owner,
        owner.to_bytes(),
        "Owner should match"
    );
    println!("Counter created with count=0");

    // ==================== PHASE 5: Increment Counter ====================
    let inc_ix = build_increment_instruction(program_id, owner, counter_pda);

    rpc.create_and_send_transaction(&[inc_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("increment should succeed");

    // Verify count=1
    let account = rpc
        .get_account(counter_pda)
        .await
        .unwrap()
        .expect("Counter should exist");
    let counter: CounterState =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
    assert_eq!(counter.count, 1, "Count should be 1 after increment");
    println!("Counter incremented to count=1");

    // ==================== PHASE 6: Warp to Trigger Compression ====================
    println!("Warping forward to trigger compression...");
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // ==================== PHASE 7: Verify Account Is Compressed ====================
    shared::assert_onchain_closed(&mut rpc, &counter_pda, "Counter").await;
    println!("Counter compressed successfully!");

    // ==================== PHASE 8: Create SDK from Compressed State ====================
    let counter_interface = rpc
        .get_account_interface(&counter_pda, &program_id)
        .await
        .expect("counter should be compressed");
    assert!(
        counter_interface.is_cold(),
        "counter should be cold after warp"
    );

    let mut counter_sdk = CounterSdk::from_keyed_accounts(&[counter_interface])
        .expect("from_keyed_accounts should succeed");

    // ==================== PHASE 9: Fetch and Update SDK ====================
    let accounts_to_fetch =
        counter_sdk.get_accounts_to_update(&CounterInstruction::Increment);
    let keyed_accounts = rpc
        .get_multiple_account_interfaces(&accounts_to_fetch)
        .await
        .expect("get_multiple_account_interfaces should succeed");

    counter_sdk
        .update(&keyed_accounts)
        .expect("sdk.update should succeed");

    // ==================== PHASE 10: Build Load Instructions ====================
    let all_specs =
        counter_sdk.get_specs_for_instruction(&CounterInstruction::Increment);

    let load_ixs = create_load_instructions::<LightAccountVariant, LightProgramTest>(
        &all_specs,
        payer.pubkey(),
        env.config_pda,
        &rpc,
    )
    .await
    .expect("create_load_instructions should succeed");

    // ==================== PHASE 11: Execute Decompression ====================
    println!("Decompressing counter...");
    rpc.create_and_send_transaction(&load_ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // ==================== PHASE 12: Verify Counter Is Restored ====================
    shared::assert_onchain_exists(&mut rpc, &counter_pda, "Counter").await;

    // Verify state is preserved
    let account = rpc
        .get_account(counter_pda)
        .await
        .unwrap()
        .expect("Counter should exist after decompression");
    let counter: CounterState =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
    assert_eq!(
        counter.count, 1,
        "Count should still be 1 after decompression"
    );
    assert_eq!(
        counter.owner,
        owner.to_bytes(),
        "Owner should be preserved"
    );
    println!("Counter decompressed with count=1 preserved!");

    // ==================== PHASE 13: Increment After Decompression ====================
    let inc_ix_2 = build_increment_instruction(program_id, owner, counter_pda);

    rpc.create_and_send_transaction(&[inc_ix_2], &payer.pubkey(), &[&payer])
        .await
        .expect("increment after decompression should succeed");

    // Verify count=2
    let account = rpc
        .get_account(counter_pda)
        .await
        .unwrap()
        .expect("Counter should exist");
    let counter: CounterState =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
    assert_eq!(
        counter.count, 2,
        "Count should be 2 after second increment"
    );

    println!("Full lifecycle test completed successfully!");
}
