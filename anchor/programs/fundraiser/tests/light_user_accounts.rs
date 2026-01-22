//! Integration tests for the fundraiser program with Light mints and Light ATAs.
//!
//! Token Configuration:
//! - Mint Type: Light (via CreateMint instruction)
//! - User Accounts: Light token accounts (contributors)
//! - Vault Account: Light Protocol token account
//!
//! This is the pure Light-to-Light test where everything uses compressed tokens.

mod common;

use common::{create_test_rpc, run_fundraiser_full_flow, setup_fundraiser_test, TokenConfig};

/// Test the full fundraiser flow with Light tokens
#[tokio::test]
async fn test_fundraiser_full_flow_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_fundraiser_test(&mut rpc, TokenConfig::Light).await;
    run_fundraiser_full_flow(&mut rpc, &ctx).await;
}

/// Test fundraiser initialization with Light tokens
#[tokio::test]
async fn test_initialize_fundraiser_light() {
    use common::initialize_fundraiser;
    use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
    use shared_test_utils::helpers::verify_light_token_balance;

    let mut rpc = create_test_rpc().await;
    let ctx = setup_fundraiser_test(&mut rpc, TokenConfig::Light).await;

    // Get proof for creating vault
    let proof_result = get_create_accounts_proof(
        &rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(ctx.vault_pda)],
    )
    .await
    .unwrap();

    // Initialize fundraiser
    initialize_fundraiser(&mut rpc, &ctx, proof_result).await;

    // Verify vault was created
    verify_light_token_balance(&mut rpc, ctx.vault_pda, 0, "vault").await;

    println!("=== Initialize fundraiser test completed successfully! ===");
}
