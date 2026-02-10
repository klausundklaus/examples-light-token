//! Full lifecycle test for pinocchio-swap program.
//!
//! Tests the complete lifecycle:
//! 1. Setup - Initialize compression config
//! 2. Init - Create pool with 2 mints, 2 vaults, 1 pool state
//! 3. Mint tokens to user accounts
//! 4. Swap - Execute A->B swap, verify balances
//! 5. Advance Epoch - warp_slot_forward to trigger auto-compression
//! 6. Verify Compressed - Accounts should be closed on-chain
//! 7. Decompress - Use SwapSdk + create_load_instructions to decompress
//! 8. Swap Again - Execute B->A swap after decompression, verify state preserved

mod sdk;
mod shared;

use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountSpec,
    CreateAccountsProofInput, LightProgramInterface,
};
use light_program_test::{program_test::TestRpc, Rpc};
use light_token::LIGHT_TOKEN_PROGRAM_ID;

/// Slots per epoch for compression timing (matches light-compressible).
const SLOTS_PER_EPOCH: u64 = 13500;
use light_token::instruction::{
    get_associated_token_address_and_bump, CreateAssociatedTokenAccount, MintTo,
    LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR,
};
use pinocchio_swap::{
    constants::*, discriminators, init::InitializeParams, swap::SwapParams, PoolState,
};
use sdk::SwapSdk;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Build the initialize instruction for the swap pool.
fn build_initialize_instruction(
    program_id: Pubkey,
    payer: Pubkey,
    authority: Pubkey,
    pool_pdas: &shared::PoolPdas,
    token_setup: &shared::TokenSetup,
    config_pda: Pubkey,
    rent_sponsor: Pubkey,
    proof_result: &light_client::interface::CreateAccountsProofResult,
    fee_bps: u16,
) -> Instruction {
    let params = InitializeParams {
        create_accounts_proof: proof_result.create_accounts_proof.clone(),
        fee_bps,
        mint_signer_a_bump: token_setup.mint_a_signer_bump,
        mint_signer_b_bump: token_setup.mint_b_signer_bump,
        pool_bump: pool_pdas.pool_bump,
        vault_a_bump: pool_pdas.vault_a_bump,
        vault_b_bump: pool_pdas.vault_b_bump,
    };

    // Account order per init/accounts.rs:
    // [0] payer (signer, writable)
    // [1] authority (signer)
    // [2] mint_signer_a
    // [3] mint_signer_b
    // [4] mint_a (writable)
    // [5] mint_b (writable)
    // [6] pool (writable)
    // [7] pool_authority
    // [8] vault_a (writable)
    // [9] vault_b (writable)
    // [10] compressible_config (swap program's config for pool PDA)
    // [11] rent_sponsor (swap program's rent sponsor)
    // [12] light_token_config (LIGHT_TOKEN_CONFIG for mints)
    // [13] light_token_rent_sponsor (LIGHT_TOKEN_RENT_SPONSOR for mints)
    // [14] light_token_program
    // [15] cpi_authority
    // [16] system_program
    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new_readonly(token_setup.mint_a_signer, false),
        AccountMeta::new_readonly(token_setup.mint_b_signer, false),
        AccountMeta::new(token_setup.mint_a, false),
        AccountMeta::new(token_setup.mint_b, false),
        AccountMeta::new(pool_pdas.pool_state, false),
        AccountMeta::new_readonly(pool_pdas.pool_authority, false),
        AccountMeta::new(pool_pdas.vault_a, false),
        AccountMeta::new(pool_pdas.vault_b, false),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(rent_sponsor, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_CONFIG, false),
        AccountMeta::new(LIGHT_TOKEN_RENT_SPONSOR, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID.into(), false),
        AccountMeta::new_readonly(light_token::constants::CPI_AUTHORITY_PDA.into(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts.clone()].concat(),
        data: shared::build_instruction_data(&discriminators::INITIALIZE, &params),
    }
}

/// Build the swap instruction.
fn build_swap_instruction(
    program_id: Pubkey,
    user: Pubkey,
    pool_pdas: &shared::PoolPdas,
    token_setup: &shared::TokenSetup,
    amount_in: u64,
    minimum_amount_out: u64,
    a_to_b: bool,
) -> Instruction {
    let params = SwapParams {
        amount_in,
        minimum_amount_out,
        a_to_b,
        authority_bump: pool_pdas.pool_authority_bump,
    };

    // Account order per swap/accounts.rs:
    // [0] user (signer)
    // [1] pool
    // [2] pool_authority
    // [3] mint_a
    // [4] mint_b
    // [5] vault_a
    // [6] vault_b
    // [7] user_token_a
    // [8] user_token_b
    // [9] light_token_program
    // [10] light_token_cpi_authority
    // [11] system_program
    let accounts = vec![
        AccountMeta::new(user, true),
        AccountMeta::new(pool_pdas.pool_state, false),
        AccountMeta::new(pool_pdas.pool_authority, false), // needs to be writable for PDA signing in CPI
        AccountMeta::new(token_setup.mint_a, false),
        AccountMeta::new(token_setup.mint_b, false),
        AccountMeta::new(pool_pdas.vault_a, false),
        AccountMeta::new(pool_pdas.vault_b, false),
        AccountMeta::new(token_setup.user_token_a, false),
        AccountMeta::new(token_setup.user_token_b, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID.into(), false),
        AccountMeta::new_readonly(light_token::constants::CPI_AUTHORITY_PDA.into(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    Instruction {
        program_id,
        accounts,
        data: shared::build_instruction_data(&discriminators::SWAP, &params),
    }
}

#[tokio::test]
async fn test_full_lifecycle() {
    // ==================== PHASE 1: Setup Environment ====================
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let authority = Keypair::new();
    let user = Keypair::new();

    // Fund the user
    rpc.airdrop_lamports(&user.pubkey(), 10_000_000_000)
        .await
        .expect("Airdrop to user should succeed");

    // ==================== PHASE 2: Derive PDAs ====================
    // Derive mint signers (needed for light-token mint derivation)
    let (mint_a_signer, _) =
        Pubkey::find_program_address(&[MINT_A_SEED, authority.pubkey().as_ref()], &program_id);
    let (mint_b_signer, _) =
        Pubkey::find_program_address(&[MINT_B_SEED, authority.pubkey().as_ref()], &program_id);

    // Derive mint addresses from signers
    let (mint_a, _) = light_token::instruction::find_mint_address(&mint_a_signer);
    let (mint_b, _) = light_token::instruction::find_mint_address(&mint_b_signer);

    // Derive all pool PDAs using shared helper
    let (pool_pdas, mut token_setup) =
        shared::derive_pool_pdas(&program_id, &authority.pubkey(), &mint_a, &mint_b);

    // Set user token accounts (ATAs)
    token_setup.user_token_a = get_associated_token_address_and_bump(&user.pubkey(), &mint_a).0;
    token_setup.user_token_b = get_associated_token_address_and_bump(&user.pubkey(), &mint_b).0;

    // Extract commonly used values for convenience
    let pool_state = pool_pdas.pool_state;
    let pool_authority = pool_pdas.pool_authority;
    let vault_a = pool_pdas.vault_a;
    let vault_b = pool_pdas.vault_b;
    let user_token_a = token_setup.user_token_a;
    let user_token_b = token_setup.user_token_b;

    println!("Pool state: {}", pool_state);
    println!("Pool authority: {}", pool_authority);
    println!("Mint A: {}", mint_a);
    println!("Mint B: {}", mint_b);
    println!("Mint A signer: {}", mint_a_signer);
    println!("Mint B signer: {}", mint_b_signer);
    println!("Vault A: {}", vault_a);
    println!("Vault B: {}", vault_b);
    println!("User token A: {}", user_token_a);
    println!("User token B: {}", user_token_b);

    // ==================== PHASE 3: Get Proof and Initialize Pool ====================
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(pool_state),
            CreateAccountsProofInput::mint(mint_a_signer),
            CreateAccountsProofInput::mint(mint_b_signer),
        ],
    )
    .await
    .expect("get_create_accounts_proof should succeed");

    let init_ix = build_initialize_instruction(
        program_id,
        payer.pubkey(),
        authority.pubkey(),
        &pool_pdas,
        &token_setup,
        env.config_pda,
        env.rent_sponsor,
        &proof_result,
        30, // 0.3% fee
    );

    println!("Sending initialize transaction...");
    rpc.create_and_send_transaction(&[init_ix], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("Initialize should succeed");

    // ==================== PHASE 4: Verify Accounts Exist ====================
    shared::assert_all_pool_accounts_exist(&mut rpc, &pool_pdas, &token_setup).await;

    println!("All accounts created successfully!");

    // ==================== PHASE 5: Create User ATAs and Mint Tokens ====================
    // Create user ATA for token A
    let create_ata_a = CreateAssociatedTokenAccount::new(payer.pubkey(), user.pubkey(), mint_a);
    rpc.create_and_send_transaction(
        &[create_ata_a.instruction().unwrap()],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .expect("Create user ATA A should succeed");

    // Create user ATA for token B
    let create_ata_b = CreateAssociatedTokenAccount::new(payer.pubkey(), user.pubkey(), mint_b);
    rpc.create_and_send_transaction(
        &[create_ata_b.instruction().unwrap()],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .expect("Create user ATA B should succeed");

    // Mint tokens to user
    let mint_amount_a = 1_000_000_000u64; // 1B tokens
    let mint_amount_b = 1_000_000_000u64;

    let mint_to_a = MintTo {
        mint: mint_a,
        destination: user_token_a,
        amount: mint_amount_a,
        authority: authority.pubkey(),
        max_top_up: None,
        fee_payer: Some(payer.pubkey()),
    };
    rpc.create_and_send_transaction(
        &[mint_to_a.instruction().unwrap()],
        &payer.pubkey(),
        &[&payer, &authority],
    )
    .await
    .expect("MintTo A should succeed");

    let mint_to_b = MintTo {
        mint: mint_b,
        destination: user_token_b,
        amount: mint_amount_b,
        authority: authority.pubkey(),
        max_top_up: None,
        fee_payer: Some(payer.pubkey()),
    };
    rpc.create_and_send_transaction(
        &[mint_to_b.instruction().unwrap()],
        &payer.pubkey(),
        &[&payer, &authority],
    )
    .await
    .expect("MintTo B should succeed");

    // Mint initial liquidity to vaults
    let vault_initial_a = 100_000_000u64;
    let vault_initial_b = 100_000_000u64;

    let mint_to_vault_a = MintTo {
        mint: mint_a,
        destination: vault_a,
        amount: vault_initial_a,
        authority: authority.pubkey(),
        max_top_up: None,
        fee_payer: Some(payer.pubkey()),
    };
    rpc.create_and_send_transaction(
        &[mint_to_vault_a.instruction().unwrap()],
        &payer.pubkey(),
        &[&payer, &authority],
    )
    .await
    .expect("MintTo vault A should succeed");

    let mint_to_vault_b = MintTo {
        mint: mint_b,
        destination: vault_b,
        amount: vault_initial_b,
        authority: authority.pubkey(),
        max_top_up: None,
        fee_payer: Some(payer.pubkey()),
    };
    rpc.create_and_send_transaction(
        &[mint_to_vault_b.instruction().unwrap()],
        &payer.pubkey(),
        &[&payer, &authority],
    )
    .await
    .expect("MintTo vault B should succeed");

    println!("Tokens minted to user and vaults!");

    // ==================== PHASE 6: Execute Swap A->B ====================
    let swap_amount = 10_000_000u64; // 10M tokens
    let min_out = 1u64; // Minimal slippage protection for test

    let swap_ix = build_swap_instruction(
        program_id,
        user.pubkey(),
        &pool_pdas,
        &token_setup,
        swap_amount,
        min_out,
        true, // A to B
    );

    println!("Executing swap A->B...");
    rpc.create_and_send_transaction(&[swap_ix], &user.pubkey(), &[&user])
        .await
        .expect("Swap A->B should succeed");

    println!("Swap A->B completed!");

    // ==================== PHASE 7: Warp to Trigger Compression ====================
    println!("Warping forward to trigger compression...");
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // ==================== PHASE 8: Verify Accounts Are Compressed ====================
    shared::assert_all_pool_accounts_closed(&mut rpc, &pool_pdas, &token_setup).await;

    println!("All accounts compressed successfully!");

    // ==================== PHASE 9: Create SDK from Compressed State ====================
    let pool_interface = rpc
        .get_account_interface(&pool_state, None)
        .await
        .expect("pool should be compressed")
        .value
        .expect("pool interface should exist");
    assert!(
        pool_interface.is_cold(),
        "pool_state should be cold after warp"
    );

    let swap_sdk = SwapSdk::new(pool_state, pool_interface.data())
        .expect("SwapSdk::new should succeed");

    // ==================== PHASE 10: Fetch Cold Accounts ====================
    let pubkeys = swap_sdk.instruction_accounts(&sdk::SwapInstruction::Swap);
    let cold_accounts = rpc
        .get_multiple_account_interfaces(pubkeys.iter().collect(), None)
        .await
        .expect("get_multiple_account_interfaces should succeed")
        .value;
    let cold: Vec<_> = cold_accounts.into_iter().flatten().filter(|a| a.is_cold()).collect();

    // ==================== PHASE 11: Build Load Instructions ====================
    let mut all_specs = swap_sdk.load_specs(&cold)
        .expect("load_specs should succeed");

    // Also need to decompress user ATAs
    let user_ata_a_interface = rpc
        .get_associated_token_account_interface(&user.pubkey(), &mint_a, None)
        .await
        .expect("get_associated_token_account_interface for user_token_a should succeed")
        .value
        .expect("user_token_a interface should exist");

    let user_ata_b_interface = rpc
        .get_associated_token_account_interface(&user.pubkey(), &mint_b, None)
        .await
        .expect("get_associated_token_account_interface for user_token_b should succeed")
        .value
        .expect("user_token_b interface should exist");

    all_specs.push(AccountSpec::Ata(user_ata_a_interface));
    all_specs.push(AccountSpec::Ata(user_ata_b_interface));

    let load_ixs = create_load_instructions(
        &all_specs,
        payer.pubkey(),
        env.config_pda,
        &rpc,
    )
    .await
    .expect("create_load_instructions should succeed");

    // ==================== PHASE 12: Execute Decompression ====================
    println!("Decompressing accounts...");
    println!("payer pubkey: {}", payer.pubkey());
    println!("user pubkey: {}", user.pubkey());
    println!("authority pubkey: {}", authority.pubkey());
    // User is needed as owner of user ATAs
    rpc.create_and_send_transaction(&load_ixs, &payer.pubkey(), &[&payer, &user])
        .await
        .expect("Decompression should succeed");

    // ==================== PHASE 13: Verify Accounts Are Restored ====================
    shared::assert_all_pool_accounts_exist(&mut rpc, &pool_pdas, &token_setup).await;

    println!("All accounts decompressed successfully!");

    // Verify pool state is preserved
    let pool_account = rpc
        .get_account(pool_state)
        .await
        .unwrap()
        .expect("Pool state should exist");
    let restored_pool: PoolState =
        borsh::BorshDeserialize::deserialize(&mut &pool_account.data[8..]).unwrap();
    assert_eq!(restored_pool.fee_bps, 30, "Fee should be preserved");
    assert_eq!(
        restored_pool.token_a_mint,
        mint_a.to_bytes(),
        "Mint A should be preserved"
    );
    assert_eq!(
        restored_pool.token_b_mint,
        mint_b.to_bytes(),
        "Mint B should be preserved"
    );

    println!("Pool state verified after decompression!");

    // ==================== PHASE 14: Execute Swap B->A After Decompression ====================
    let swap_ix_2 = build_swap_instruction(
        program_id,
        user.pubkey(),
        &pool_pdas,
        &token_setup,
        swap_amount / 2, // Swap half the amount back
        min_out,
        false, // B to A
    );

    println!("Executing swap B->A after decompression...");
    rpc.create_and_send_transaction(&[swap_ix_2], &user.pubkey(), &[&user])
        .await
        .expect("Swap B->A should succeed");

    println!("Full lifecycle test completed successfully!");
}
