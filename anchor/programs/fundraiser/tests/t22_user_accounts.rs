//! Integration tests for the fundraiser program with Token-2022 mints and T22 ATAs.
//!
//! Token Configuration:
//! - Mint Type: Token-2022 (spl_token_2022::ID)
//! - User Accounts: Token-2022 ATAs (contributors)
//! - Vault Account: Light Protocol token account

mod common;

use common::{create_test_rpc, run_fundraiser_full_flow, setup_fundraiser_test, TokenConfig};

/// Test the full fundraiser flow with Token-2022 tokens
#[tokio::test]
async fn test_fundraiser_full_flow_t22() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_fundraiser_test(&mut rpc, TokenConfig::Token2022).await;
    run_fundraiser_full_flow(&mut rpc, &ctx).await;
}

/// Test fundraiser initialization with Token-2022 tokens
#[tokio::test]
async fn test_initialize_fundraiser_t22() {
    use common::{initialize_fundraiser, setup_fundraiser_test};
    use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
    use shared_test_utils::helpers::verify_light_token_balance;

    let mut rpc = create_test_rpc().await;
    let ctx = setup_fundraiser_test(&mut rpc, TokenConfig::Token2022).await;

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
