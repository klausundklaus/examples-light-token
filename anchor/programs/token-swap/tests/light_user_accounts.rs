//! Integration tests for the token-swap AMM program with Light mints and Light ATAs.
//!
//! Token Configuration:
//! - Mint Type: Light (via CreateMint instruction)
//! - User Accounts: Light token accounts
//! - Pool Accounts: Light Protocol token accounts
//!
//! This is the pure Light-to-Light test where everything uses compressed tokens.

mod common;

use common::{create_amm, create_test_rpc, run_amm_full_flow, setup_amm_test, TokenConfig};

#[tokio::test]
async fn test_amm_full_flow_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Light).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}

#[tokio::test]
async fn test_create_amm_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::Light).await;

    // Create AMM with 0% fee
    create_amm(&mut rpc, &ctx, 0).await;

    println!("AMM with 0% fee created successfully");
    println!("=== Initialize AMM test completed successfully! ===");
}
