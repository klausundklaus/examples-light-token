//! Integration tests for the token-swap AMM program with T22 mints and Light user accounts.
//!
//! Token Configuration:
//! - Mint Type: Token-2022 (spl_token_2022::ID)
//! - User Accounts: Light token accounts (offchain T22->Light conversion before program interaction)
//! - Pool Accounts: Light Protocol token accounts
//!
//! Test Flow:
//! 1. Create T22 mint and temporary T22 ATA
//! 2. Mint tokens to temporary ATA
//! 3. Create Light token account for user
//! 4. Create SPL interface PDA (works with T22)
//! 5. Transfer from T22 ATA to Light account (offchain conversion using `transfer_spl_to_light`)
//! 6. User has Light account with tokens - interact with program using Light-to-Light

mod common;

use common::{create_test_rpc, run_amm_full_flow, setup_amm_test, TokenConfig};

#[tokio::test]
async fn test_amm_full_flow_light_t22() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_amm_test(&mut rpc, TokenConfig::LightT22).await;
    run_amm_full_flow(&mut rpc, &ctx).await;
}
