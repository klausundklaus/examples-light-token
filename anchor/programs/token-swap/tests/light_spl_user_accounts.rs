//! Integration tests for the token-swap AMM program with SPL mints and Light user accounts.
//!
//! Token Configuration:
//! - Mint Type: SPL (token::ID)
//! - User Accounts: Light token accounts (offchain SPL->Light conversion before program interaction)
//! - Pool Accounts: Light Protocol token accounts
//!
//! Test Flow:
//! 1. Create SPL mint and temporary SPL ATA
//! 2. Mint tokens to temporary ATA
//! 3. Create Light token account for user
//! 4. Create SPL interface PDA
//! 5. Transfer from SPL ATA to Light account (offchain conversion using `transfer_spl_to_light`)
//! 6. User has Light account with tokens - interact with program using Light-to-Light

mod common;

use common::{create_test_rpc, run_amm_full_flow, setup_amm_test, TokenConfig};

#[tokio::test]
async fn test_amm_full_flow_light_spl() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::LightSpl).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}
