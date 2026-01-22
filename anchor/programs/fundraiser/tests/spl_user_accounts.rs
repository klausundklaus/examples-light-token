//! Integration tests for the fundraiser program with SPL mints and SPL ATAs.
//!
//! Token Configuration:
//! - Mint Type: SPL (token::ID)
//! - User Accounts: SPL ATAs (contributors)
//! - Vault Account: Light Protocol token account

mod common;

use common::{create_test_rpc, run_fundraiser_full_flow, setup_fundraiser_test, TokenConfig};

/// Test the full fundraiser flow with SPL tokens
#[tokio::test]
async fn test_fundraiser_full_flow_spl() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_fundraiser_test(&mut rpc, TokenConfig::Spl).await;
    run_fundraiser_full_flow(&mut rpc, &ctx).await;
}

/// Test fundraiser initialization with SPL tokens
#[tokio::test]
async fn test_initialize_fundraiser_spl() {
    use common::{initialize_fundraiser, setup_fundraiser_test};
    use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
    use shared_test_utils::helpers::verify_light_token_balance;

    let mut rpc = create_test_rpc().await;
    let ctx = setup_fundraiser_test(&mut rpc, TokenConfig::Spl).await;

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
