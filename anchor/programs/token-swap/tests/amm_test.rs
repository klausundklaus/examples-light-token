//! Integration tests for the token-swap AMM program with Light Protocol pool accounts.
//!
//! This test demonstrates a full AMM flow using:
//! - Standard SPL mints for token A and token B
//! - Standard SPL token accounts for depositor/trader
//! - Light Protocol token accounts for pool_account_a and pool_account_b
//! - Standard SPL mint for liquidity tokens

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token;
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
use light_token::constants::CPI_AUTHORITY_PDA;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};
use light_token::spl_interface::{find_spl_interface_pda, CreateSplInterfacePda};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022::pod::PodAccount;

/// Verify Light Token account balance.
async fn verify_light_token_balance<R: Rpc>(rpc: &mut R, account: Pubkey, expected: u64, name: &str) {
    let account_data = rpc.get_account(account).await.unwrap();

    if let Some(data) = account_data {
        // Light Token accounts have first 165 bytes as SPL-compatible token account data
        if data.data.len() >= 165 {
            let token_state =
                spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&data.data[..165]).unwrap();
            let actual = u64::from(token_state.amount);
            println!("{}: balance = {} (expected {})", name, actual, expected);
            assert_eq!(
                actual, expected,
                "{} balance mismatch: expected {}, got {}",
                name, expected, actual
            );
        } else {
            panic!(
                "{}: account data too short: {} bytes",
                name,
                data.data.len()
            );
        }
    } else if expected == 0 {
        println!("{}: account not found (expected 0 balance)", name);
    } else {
        panic!(
            "{}: account not found but expected balance {}",
            name, expected
        );
    }
}

/// Test the full AMM flow: create_amm, create_pool, deposit, swap, withdraw
#[tokio::test]
async fn test_amm_full_flow() {
    let program_id = swap_example::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("swap_example", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Setup program data for rent-free config
    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize rent-free config
    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
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

    println!("Rent-free config initialized at: {:?}", config_pda);

    // Create depositor/trader keypair
    let depositor = Keypair::new();
    rpc.airdrop_lamports(&depositor.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // === STEP 1: Create SPL mints for token A and token B ===
    println!("\n=== Creating SPL mints ===");

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    // Create mint A
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(82)
        .await
        .unwrap();

    let create_mint_a_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint_a.pubkey(),
        rent,
        82,
        &token::ID,
    );
    let init_mint_a_ix = token::spl_token::instruction::initialize_mint(
        &token::ID,
        &mint_a.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint_a_ix, init_mint_a_ix],
        &payer.pubkey(),
        &[&payer, &mint_a],
    )
    .await
    .expect("Create mint A should succeed");

    // Create mint B
    let create_mint_b_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint_b.pubkey(),
        rent,
        82,
        &token::ID,
    );
    let init_mint_b_ix = token::spl_token::instruction::initialize_mint(
        &token::ID,
        &mint_b.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint_b_ix, init_mint_b_ix],
        &payer.pubkey(),
        &[&payer, &mint_b],
    )
    .await
    .expect("Create mint B should succeed");

    println!("Mint A: {:?}", mint_a.pubkey());
    println!("Mint B: {:?}", mint_b.pubkey());

    // === STEP 1.5: Create SPL interface PDAs for both mints ===
    // These are required for SPL<->Light token transfers
    println!("\n=== Creating SPL interface PDAs ===");

    let (spl_interface_pda_a, _) = find_spl_interface_pda(&mint_a.pubkey(), false);
    let (spl_interface_pda_b, _) = find_spl_interface_pda(&mint_b.pubkey(), false);

    let create_spl_interface_a_ix =
        CreateSplInterfacePda::new(payer.pubkey(), mint_a.pubkey(), token::ID, false).instruction();
    let create_spl_interface_b_ix =
        CreateSplInterfacePda::new(payer.pubkey(), mint_b.pubkey(), token::ID, false).instruction();

    rpc.create_and_send_transaction(
        &[create_spl_interface_a_ix, create_spl_interface_b_ix],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .expect("Create SPL interface PDAs should succeed");

    println!("SPL interface PDA A: {:?}", spl_interface_pda_a);
    println!("SPL interface PDA B: {:?}", spl_interface_pda_b);

    // === STEP 2: Create SPL token accounts for depositor ===
    println!("\n=== Creating depositor token accounts ===");

    let depositor_ata_a = get_associated_token_address(&depositor.pubkey(), &mint_a.pubkey());
    let depositor_ata_b = get_associated_token_address(&depositor.pubkey(), &mint_b.pubkey());

    let create_depositor_ata_a_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &depositor.pubkey(),
            &mint_a.pubkey(),
            &token::ID,
        );
    let create_depositor_ata_b_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &depositor.pubkey(),
            &mint_b.pubkey(),
            &token::ID,
        );

    rpc.create_and_send_transaction(
        &[create_depositor_ata_a_ix, create_depositor_ata_b_ix],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .expect("Create depositor ATAs should succeed");

    println!("Depositor ATA A: {:?}", depositor_ata_a);
    println!("Depositor ATA B: {:?}", depositor_ata_b);

    // === STEP 3: Mint tokens to depositor ===
    println!("\n=== Minting tokens to depositor ===");

    let amount_a = 10_000_000_000_000u64; // 10,000 tokens with 9 decimals
    let amount_b = 10_000_000_000_000u64;

    let mint_to_a_ix = token::spl_token::instruction::mint_to(
        &token::ID,
        &mint_a.pubkey(),
        &depositor_ata_a,
        &payer.pubkey(),
        &[],
        amount_a,
    )
    .unwrap();
    let mint_to_b_ix = token::spl_token::instruction::mint_to(
        &token::ID,
        &mint_b.pubkey(),
        &depositor_ata_b,
        &payer.pubkey(),
        &[],
        amount_b,
    )
    .unwrap();

    rpc.create_and_send_transaction(&[mint_to_a_ix, mint_to_b_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Mint tokens should succeed");

    println!("Minted {} token A to depositor", amount_a);
    println!("Minted {} token B to depositor", amount_b);

    // === STEP 4: Create AMM ===
    println!("\n=== Creating AMM ===");

    let amm_id = Pubkey::new_unique();
    let fee = 250u16; // 2.5% fee

    let (amm_pda, _amm_bump) =
        Pubkey::find_program_address(&[amm_id.as_ref()], &program_id);

    let create_amm_accounts = swap_example::accounts::CreateAmm {
        amm: amm_pda,
        admin: payer.pubkey(),
        payer: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };

    let create_amm_data = swap_example::instruction::CreateAmm { id: amm_id, fee };

    let create_amm_ix = Instruction {
        program_id,
        accounts: create_amm_accounts.to_account_metas(None),
        data: create_amm_data.data(),
    };

    rpc.create_and_send_transaction(&[create_amm_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("create_amm should succeed");

    println!("AMM created at: {:?}", amm_pda);
    println!("AMM ID: {:?}", amm_id);
    println!("AMM Fee: {} basis points", fee);

    // Verify AMM state
    let amm_account = rpc
        .get_account(amm_pda)
        .await
        .unwrap()
        .expect("AMM should exist");
    assert!(!amm_account.data.is_empty(), "AMM should have data");

    // === STEP 5: Create Pool ===
    println!("\n=== Creating Pool ===");

    // Derive pool PDA
    let (pool_pda, _pool_bump) = Pubkey::find_program_address(
        &[
            amm_pda.as_ref(),
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
        ],
        &program_id,
    );

    // Derive pool authority
    let (pool_authority, _authority_bump) = Pubkey::find_program_address(
        &[
            amm_pda.as_ref(),
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
            b"authority",
        ],
        &program_id,
    );

    // Derive liquidity mint
    let (mint_liquidity, _liquidity_bump) = Pubkey::find_program_address(
        &[
            amm_pda.as_ref(),
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
            b"liquidity",
        ],
        &program_id,
    );

    // Derive pool token accounts (Light token accounts)
    let (pool_account_a, pool_a_bump) =
        Pubkey::find_program_address(&[b"pool_a", pool_pda.as_ref()], &program_id);
    let (pool_account_b, pool_b_bump) =
        Pubkey::find_program_address(&[b"pool_b", pool_pda.as_ref()], &program_id);

    println!("Pool PDA: {:?}", pool_pda);
    println!("Pool Authority: {:?}", pool_authority);
    println!("Liquidity Mint: {:?}", mint_liquidity);
    println!("Pool Account A: {:?}", pool_account_a);
    println!("Pool Account B: {:?}", pool_account_b);

    // Get proof for creating Light token accounts (PDAs for pool accounts)
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(pool_account_a),
            CreateAccountsProofInput::pda(pool_account_b),
        ],
    )
    .await
    .unwrap();

    let create_pool_accounts = swap_example::accounts::CreatePool {
        amm: amm_pda,
        pool: pool_pda,
        pool_authority,
        mint_liquidity,
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        pool_account_a,
        pool_account_b,
        fee_payer: payer.pubkey(),
        token_program: token::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        system_program: solana_sdk::system_program::ID,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
    };

    let create_pool_data = swap_example::instruction::CreatePool {
        params: swap_example::CreatePoolParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            pool_account_a_bump: pool_a_bump,
            pool_account_b_bump: pool_b_bump,
        },
    };

    let create_pool_ix = Instruction {
        program_id,
        accounts: [
            create_pool_accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: create_pool_data.data(),
    };

    rpc.create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("create_pool should succeed");

    println!("Pool created successfully!");

    // Verify pool state
    let pool_account = rpc
        .get_account(pool_pda)
        .await
        .unwrap()
        .expect("Pool should exist");
    assert!(!pool_account.data.is_empty(), "Pool should have data");

    // Verify Light token accounts were created
    verify_light_token_balance(&mut rpc, pool_account_a, 0, "pool_account_a (initial)").await;
    verify_light_token_balance(&mut rpc, pool_account_b, 0, "pool_account_b (initial)").await;

    // === STEP 6: Deposit Initial Liquidity ===
    println!("\n=== Depositing Initial Liquidity ===");

    // Use smaller amounts to avoid I64F64 overflow in liquidity calculation
    // sqrt(a * b) must fit in I64F64 (max ~9.2 * 10^18)
    // 1 token = 10^9 units, so 10^9 * 10^9 = 10^18 which is within range
    let deposit_amount_a = 1_000_000_000u64; // 1 token with 9 decimals
    let deposit_amount_b = 1_000_000_000u64;

    // Create depositor's liquidity ATA
    let depositor_liquidity_ata =
        get_associated_token_address(&depositor.pubkey(), &mint_liquidity);

    let deposit_accounts = swap_example::accounts::DepositLiquidity {
        pool: pool_pda,
        pool_authority,
        depositor: depositor.pubkey(),
        mint_liquidity,
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        pool_account_a,
        pool_account_b,
        depositor_account_liquidity: depositor_liquidity_ata,
        depositor_account_a: depositor_ata_a,
        depositor_account_b: depositor_ata_b,
        payer: payer.pubkey(),
        token_program: token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a,
        spl_interface_pda_b,
    };

    let deposit_data = swap_example::instruction::DepositLiquidity {
        amount_a: deposit_amount_a,
        amount_b: deposit_amount_b,
    };

    let deposit_ix = Instruction {
        program_id,
        accounts: deposit_accounts.to_account_metas(None),
        data: deposit_data.data(),
    };

    rpc.create_and_send_transaction(&[deposit_ix], &payer.pubkey(), &[&payer, &depositor])
        .await
        .expect("deposit_liquidity should succeed");

    println!("Deposited {} token A and {} token B", deposit_amount_a, deposit_amount_b);

    // Verify pool balances after deposit
    verify_light_token_balance(&mut rpc, pool_account_a, deposit_amount_a, "pool_account_a (after deposit)").await;
    verify_light_token_balance(&mut rpc, pool_account_b, deposit_amount_b, "pool_account_b (after deposit)").await;

    // Check liquidity tokens minted
    // LP tokens = sqrt(a*b) - MINIMUM_LIQUIDITY
    // sqrt(1_000_000_000 * 1_000_000_000) - 100 = 1_000_000_000 - 100 = 999_999_900
    let expected_lp = 1_000_000_000u64 - 100; // MINIMUM_LIQUIDITY = 100
    let lp_account = rpc
        .get_account(depositor_liquidity_ata)
        .await
        .unwrap()
        .expect("LP account should exist");
    if lp_account.data.len() >= 72 {
        let lp_token =
            spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&lp_account.data[..165])
                .unwrap();
        let lp_balance = u64::from(lp_token.amount);
        println!("LP tokens minted: {} (expected ~{})", lp_balance, expected_lp);
        assert!(lp_balance > 0, "Should have received LP tokens");
    }

    // === STEP 7: Swap A for B ===
    println!("\n=== Swapping A for B ===");

    // Create a separate trader for swap testing
    let trader = Keypair::new();
    rpc.airdrop_lamports(&trader.pubkey(), 5_000_000_000)
        .await
        .unwrap();

    // Create trader token accounts
    let trader_ata_a = get_associated_token_address(&trader.pubkey(), &mint_a.pubkey());
    let trader_ata_b = get_associated_token_address(&trader.pubkey(), &mint_b.pubkey());

    let create_trader_ata_a_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &trader.pubkey(),
            &mint_a.pubkey(),
            &token::ID,
        );
    let create_trader_ata_b_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &trader.pubkey(),
            &mint_b.pubkey(),
            &token::ID,
        );

    rpc.create_and_send_transaction(
        &[create_trader_ata_a_ix, create_trader_ata_b_ix],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .expect("Create trader ATAs should succeed");

    // Mint some token A to trader
    let trader_initial_a = 100_000_000u64; // 0.1 tokens
    let mint_to_trader_ix = token::spl_token::instruction::mint_to(
        &token::ID,
        &mint_a.pubkey(),
        &trader_ata_a,
        &payer.pubkey(),
        &[],
        trader_initial_a,
    )
    .unwrap();

    rpc.create_and_send_transaction(&[mint_to_trader_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Mint to trader should succeed");

    println!("Trader has {} token A", trader_initial_a);

    // Perform swap: A -> B
    let swap_input = 10_000_000u64; // 0.01 tokens
    let min_output = 1u64; // Accept any output for test

    let swap_accounts = swap_example::accounts::SwapExactTokensForTokens {
        amm: amm_pda,
        pool: pool_pda,
        pool_authority,
        trader: trader.pubkey(),
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        pool_account_a,
        pool_account_b,
        trader_account_a: trader_ata_a,
        trader_account_b: trader_ata_b,
        payer: payer.pubkey(),
        token_program: token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a,
        spl_interface_pda_b,
    };

    let swap_data = swap_example::instruction::SwapExactTokensForTokens {
        swap_a: true, // Swap A for B
        input_amount: swap_input,
        min_output_amount: min_output,
    };

    let swap_ix = Instruction {
        program_id,
        accounts: swap_accounts.to_account_metas(None),
        data: swap_data.data(),
    };

    rpc.create_and_send_transaction(&[swap_ix], &payer.pubkey(), &[&payer, &trader])
        .await
        .expect("swap A->B should succeed");

    println!("Swapped {} token A", swap_input);

    // Verify swap results
    let trader_a_account = rpc
        .get_account(trader_ata_a)
        .await
        .unwrap()
        .expect("Trader A account should exist");
    let trader_a_token =
        spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&trader_a_account.data[..165])
            .unwrap();
    let trader_a_balance = u64::from(trader_a_token.amount);
    println!("Trader token A after swap: {}", trader_a_balance);
    assert_eq!(
        trader_a_balance,
        trader_initial_a - swap_input,
        "Trader should have spent input tokens"
    );

    let trader_b_account = rpc
        .get_account(trader_ata_b)
        .await
        .unwrap()
        .expect("Trader B account should exist");
    let trader_b_token =
        spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&trader_b_account.data[..165])
            .unwrap();
    let trader_b_balance = u64::from(trader_b_token.amount);
    println!("Trader token B after swap: {}", trader_b_balance);
    assert!(trader_b_balance > 0, "Trader should have received token B");

    // === STEP 8: Swap B for A ===
    println!("\n=== Swapping B for A ===");

    let swap_b_input = trader_b_balance / 2; // Swap half of what we received

    let swap_b_accounts = swap_example::accounts::SwapExactTokensForTokens {
        amm: amm_pda,
        pool: pool_pda,
        pool_authority,
        trader: trader.pubkey(),
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        pool_account_a,
        pool_account_b,
        trader_account_a: trader_ata_a,
        trader_account_b: trader_ata_b,
        payer: payer.pubkey(),
        token_program: token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a,
        spl_interface_pda_b,
    };

    let swap_b_data = swap_example::instruction::SwapExactTokensForTokens {
        swap_a: false, // Swap B for A
        input_amount: swap_b_input,
        min_output_amount: 1,
    };

    let swap_b_ix = Instruction {
        program_id,
        accounts: swap_b_accounts.to_account_metas(None),
        data: swap_b_data.data(),
    };

    rpc.create_and_send_transaction(&[swap_b_ix], &payer.pubkey(), &[&payer, &trader])
        .await
        .expect("swap B->A should succeed");

    println!("Swapped {} token B", swap_b_input);

    // Verify final trader balances
    let trader_a_final = rpc
        .get_account(trader_ata_a)
        .await
        .unwrap()
        .expect("Trader A account should exist");
    let trader_a_token_final =
        spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&trader_a_final.data[..165])
            .unwrap();
    let trader_a_balance_final = u64::from(trader_a_token_final.amount);
    println!("Trader final token A: {}", trader_a_balance_final);
    assert!(
        trader_a_balance_final > trader_a_balance,
        "Trader should have gained token A from B->A swap"
    );

    // === STEP 9: Withdraw Liquidity ===
    println!("\n=== Withdrawing Liquidity ===");

    // Get current LP balance
    let lp_account_before = rpc
        .get_account(depositor_liquidity_ata)
        .await
        .unwrap()
        .expect("LP account should exist");
    let lp_token_before =
        spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&lp_account_before.data[..165])
            .unwrap();
    let lp_balance_before = u64::from(lp_token_before.amount);

    // Withdraw half of LP tokens
    let withdraw_amount = lp_balance_before / 2;

    let withdraw_accounts = swap_example::accounts::WithdrawLiquidity {
        amm: amm_pda,
        pool: pool_pda,
        pool_authority,
        depositor: depositor.pubkey(),
        mint_liquidity,
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        pool_account_a,
        pool_account_b,
        depositor_account_liquidity: depositor_liquidity_ata,
        depositor_account_a: depositor_ata_a,
        depositor_account_b: depositor_ata_b,
        payer: payer.pubkey(),
        token_program: token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a,
        spl_interface_pda_b,
    };

    let withdraw_data = swap_example::instruction::WithdrawLiquidity {
        amount: withdraw_amount,
    };

    let withdraw_ix = Instruction {
        program_id,
        accounts: withdraw_accounts.to_account_metas(None),
        data: withdraw_data.data(),
    };

    rpc.create_and_send_transaction(&[withdraw_ix], &payer.pubkey(), &[&payer, &depositor])
        .await
        .expect("withdraw_liquidity should succeed");

    println!("Withdrew {} LP tokens", withdraw_amount);

    // Verify LP tokens were burned
    let lp_account_after = rpc
        .get_account(depositor_liquidity_ata)
        .await
        .unwrap()
        .expect("LP account should exist");
    let lp_token_after =
        spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&lp_account_after.data[..165])
            .unwrap();
    let lp_balance_after = u64::from(lp_token_after.amount);
    println!(
        "LP tokens after withdrawal: {} (was {})",
        lp_balance_after, lp_balance_before
    );
    assert_eq!(
        lp_balance_after,
        lp_balance_before - withdraw_amount,
        "LP tokens should be burned"
    );

    // Verify depositor received tokens back
    let depositor_a_final = rpc
        .get_account(depositor_ata_a)
        .await
        .unwrap()
        .expect("Depositor A account should exist");
    let depositor_a_token =
        spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&depositor_a_final.data[..165])
            .unwrap();
    let depositor_a_balance = u64::from(depositor_a_token.amount);
    println!("Depositor final token A: {}", depositor_a_balance);

    let depositor_b_final = rpc
        .get_account(depositor_ata_b)
        .await
        .unwrap()
        .expect("Depositor B account should exist");
    let depositor_b_token =
        spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(&depositor_b_final.data[..165])
            .unwrap();
    let depositor_b_balance = u64::from(depositor_b_token.amount);
    println!("Depositor final token B: {}", depositor_b_balance);

    // Depositor started with 10,000 tokens each, deposited 1,000 each
    // After withdrawal, should have more than (10,000 - 1,000) = 9,000 tokens
    // because we withdrew some back
    assert!(
        depositor_a_balance > amount_a - deposit_amount_a,
        "Depositor should have received token A back"
    );
    assert!(
        depositor_b_balance > amount_b - deposit_amount_b,
        "Depositor should have received token B back"
    );

    println!("\n=== AMM full flow test completed successfully! ===");
}

/// Test AMM creation with various fee values
#[tokio::test]
async fn test_create_amm() {
    let program_id = swap_example::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("swap_example", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Setup program data for rent-free config
    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize rent-free config
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

    // Create AMM with 0% fee
    let amm_id_1 = Pubkey::new_unique();
    let (amm_pda_1, _) = Pubkey::find_program_address(&[amm_id_1.as_ref()], &program_id);

    let create_amm_accounts_1 = swap_example::accounts::CreateAmm {
        amm: amm_pda_1,
        admin: payer.pubkey(),
        payer: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };

    let create_amm_ix_1 = Instruction {
        program_id,
        accounts: create_amm_accounts_1.to_account_metas(None),
        data: swap_example::instruction::CreateAmm {
            id: amm_id_1,
            fee: 0,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[create_amm_ix_1], &payer.pubkey(), &[&payer])
        .await
        .expect("create_amm with 0% fee should succeed");

    println!("AMM with 0% fee created at: {:?}", amm_pda_1);

    // Create AMM with max fee (99.99%)
    let amm_id_2 = Pubkey::new_unique();
    let (amm_pda_2, _) = Pubkey::find_program_address(&[amm_id_2.as_ref()], &program_id);

    let create_amm_accounts_2 = swap_example::accounts::CreateAmm {
        amm: amm_pda_2,
        admin: payer.pubkey(),
        payer: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };

    let create_amm_ix_2 = Instruction {
        program_id,
        accounts: create_amm_accounts_2.to_account_metas(None),
        data: swap_example::instruction::CreateAmm {
            id: amm_id_2,
            fee: 9999, // Max valid fee
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[create_amm_ix_2], &payer.pubkey(), &[&payer])
        .await
        .expect("create_amm with 99.99% fee should succeed");

    println!("AMM with 99.99% fee created at: {:?}", amm_pda_2);

    println!("\n=== create_amm test completed successfully! ===");
}
