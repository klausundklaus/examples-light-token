//! Integration tests for the token-swap AMM program with Token-2022 mints and T22 ATAs.
//!
//! Token Configuration:
//! - Mint Type: Token-2022 (spl_token_2022::ID)
//! - User Accounts: Token-2022 ATAs
//! - Pool Accounts: Light Protocol token accounts

mod common;

use common::{create_test_rpc, run_amm_full_flow, setup_amm_test, TokenConfig};

/// Test the full AMM flow with Token-2022 tokens
#[tokio::test]
async fn test_amm_full_flow_t22() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Token2022).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// Test AMM creation with various fee values using Token-2022 tokens
#[tokio::test]
async fn test_create_amm_t22() {
    use common::{create_amm, setup_amm_test};

    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Token2022).await;

    // Create AMM with 2.5% fee
    create_amm(&mut rpc, &ctx, 250).await;
    println!("AMM with 2.5% fee created successfully");
}
