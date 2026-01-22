//! Integration tests for the token-swap AMM program with SPL mints and SPL ATAs.
//!
//! Token Configuration:
//! - Mint Type: SPL (token::ID)
//! - User Accounts: SPL ATAs
//! - Pool Accounts: Light Protocol token accounts

mod common;

use common::{create_test_rpc, run_amm_full_flow, setup_amm_test, TokenConfig};

/// Test the full AMM flow with SPL tokens
#[tokio::test]
async fn test_amm_full_flow_spl() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Spl).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

/// Test AMM creation with various fee values using SPL tokens
#[tokio::test]
async fn test_create_amm_spl() {
    use common::{create_amm, setup_amm_test};

    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Spl).await;

    // Create AMM with 0% fee
    create_amm(&mut rpc, &ctx, 0).await;
    println!("AMM with 0% fee created successfully");
}
