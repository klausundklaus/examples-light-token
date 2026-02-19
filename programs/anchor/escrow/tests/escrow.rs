//! Escrow tests for 5 token configurations. Vault is always a Light Token account.
//!
//! | Test | Mint | User accounts |
//! |------|------|---------------|
//! | `test_escrow_spl` | SPL | SPL |
//! | `test_escrow_t22` | Token 2022 | Token 2022 |
//! | `test_escrow_light` | Light Token | Light Token |
//! | `test_escrow_spl_light` | SPL | Light Token |
//! | `test_escrow_t22_light` | Token 2022 | Light Token |
//!
//! Each test also simulates the cold/hot lifecycle:
//! Light Token accounts auto-compress when sponsored rent expires. Before each
//! transaction, the test loads cold Light Token accounts to associated Light Token
//! accounts (hot balance) via per-instruction load functions.

mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::{
    create_test_rpc, create_token_account, load_accounts_make_offer, load_accounts_take_offer,
    setup_escrow_test, warp_to_compress, EscrowTestContext, TokenConfig,
};
use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
use light_token::spl_interface::find_spl_interface_pda;
use shared_test_utils::{
    helpers::verify_light_token_balance, Indexer, Rpc, TestRpc, COMPRESSIBLE_CONFIG_V1,
    CPI_AUTHORITY_PDA, LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// SPL mint with SPL associated token accounts and Light Token vault.
#[tokio::test]
async fn test_escrow_spl() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_escrow_test(&mut rpc, TokenConfig::Spl).await;
    run_escrow(&mut rpc, &ctx).await;
}

/// Token 2022 mint with Token 2022 associated token accounts and Light Token vault.
#[tokio::test]
async fn test_escrow_t22() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_escrow_test(&mut rpc, TokenConfig::Token2022).await;
    run_escrow(&mut rpc, &ctx).await;
}

/// Light Token mint + Light Token associated accounts and a Light Token vault.
#[tokio::test]
async fn test_escrow_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_escrow_test(&mut rpc, TokenConfig::Light).await;
    run_escrow(&mut rpc, &ctx).await;
}

/// SPL mint + Light Token user accounts. SPL tokens converted to Light Token
/// associated token accounts in setup via `transfer_spl_to_light`.
#[tokio::test]
async fn test_escrow_spl_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_escrow_test(&mut rpc, TokenConfig::LightSpl).await;
    run_escrow(&mut rpc, &ctx).await;
}

/// Token 2022 mint + Light Token user accounts. Token 2022 tokens converted to Light Token
/// associated token accounts in setup via `transfer_spl_to_light`.
#[tokio::test]
async fn test_escrow_t22_light() {
    let mut rpc = create_test_rpc().await;
    let ctx = setup_escrow_test(&mut rpc, TokenConfig::LightT22).await;
    run_escrow(&mut rpc, &ctx).await;
}

/// Run the full escrow flow for any token configuration: SPL, Token 2022, or Light.
///
/// 1. Create token accounts per `TokenConfig` (see `create_token_account`)
/// 2. Compress and load all accounts to active state (simulates cold/hot lifecycle)
/// 3. Make offer: deposit token A into vault, record escrow terms
/// 4. Compress and load again before take (simulates cold/hot lifecycle)
/// 5. Take offer: taker sends token B to maker, vault releases token A to taker
/// 6. Verify balances and account closure
async fn run_escrow<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &EscrowTestContext,
) {
    // Maker offers 1 token A, wants 0.5 token B in return
    let token_a_offered = 1_000_000_000u64; // 1 token (9 decimals)
    let token_b_wanted = 500_000_000u64; // 0.5 tokens
    let offer_id = 1u64;

    // Maker needs: funded token A account, empty token B account (receives payment)
    // Taker needs: empty token A account (receives offered tokens), funded token B account

    let maker_token_a = create_token_account(
        rpc,
        ctx,
        &ctx.maker,
        &ctx.mint_a_pubkey,
        ctx.light_mint_a_authority.as_ref(),
        ctx.spl_interface_a.as_ref(),
        token_a_offered,
    )
    .await;

    let maker_token_b = create_token_account(
        rpc,
        ctx,
        &ctx.maker,
        &ctx.mint_b_pubkey,
        ctx.light_mint_b_authority.as_ref(),
        ctx.spl_interface_b.as_ref(),
        0,
    )
    .await;

    let taker_token_a = create_token_account(
        rpc,
        ctx,
        &ctx.taker,
        &ctx.mint_a_pubkey,
        ctx.light_mint_a_authority.as_ref(),
        ctx.spl_interface_a.as_ref(),
        0,
    )
    .await;

    let taker_token_b = create_token_account(
        rpc,
        ctx,
        &ctx.taker,
        &ctx.mint_b_pubkey,
        ctx.light_mint_b_authority.as_ref(),
        ctx.spl_interface_b.as_ref(),
        token_b_wanted,
    )
    .await;

    verify_light_token_balance(rpc, maker_token_a, token_a_offered, "maker_token_a (initial)")
        .await;
    verify_light_token_balance(rpc, maker_token_b, 0, "maker_token_b (initial)").await;
    verify_light_token_balance(rpc, taker_token_a, 0, "taker_token_a (initial)").await;
    verify_light_token_balance(rpc, taker_token_b, token_b_wanted, "taker_token_b (initial)")
        .await;

    // Simulate hot-cold lifecycle. No-op for non-Light configs.
    warp_to_compress(rpc).await;

    // Load cold accounts referenced by make_offer to active state.
    load_accounts_make_offer(
        rpc,
        &ctx.payer,
        &ctx.maker,
        ctx.compression_config,
        &ctx.mint_a_pubkey,
        &ctx.mint_b_pubkey,
    )
    .await;

    let (offer_pda, vault_pda) = make_offer(
        rpc,
        ctx,
        maker_token_a,
        offer_id,
        token_a_offered,
        token_b_wanted,
    )
    .await;

    verify_light_token_balance(rpc, vault_pda, token_a_offered, "vault (after make_offer)").await;
    verify_light_token_balance(rpc, maker_token_a, 0, "maker_token_a (after make_offer)").await;

    let offer_account = rpc
        .get_account(offer_pda)
        .await
        .unwrap()
        .expect("Offer account should exist");
    assert!(
        !offer_account.data.is_empty(),
        "Offer account should have data"
    );

    // Simulate hot-cold lifecycle before take_offer.
    warp_to_compress(rpc).await;

    // Load cold accounts referenced by take_offer to active state.
    load_accounts_take_offer(
        rpc,
        &ctx.payer,
        &ctx.maker,
        &ctx.taker,
        &ctx.program_id,
        ctx.compression_config,
        &ctx.mint_a_pubkey,
        &ctx.mint_b_pubkey,
        offer_pda,
        vault_pda,
    )
    .await;

    take_offer(
        rpc,
        ctx,
        taker_token_a,
        taker_token_b,
        maker_token_b,
        offer_pda,
        vault_pda,
    )
    .await;

    // Maker: gave token A, received token B
    // Taker: gave token B, received token A

    verify_light_token_balance(rpc, maker_token_a, 0, "maker_token_a (final)").await;
    verify_light_token_balance(rpc, maker_token_b, token_b_wanted, "maker_token_b (final)").await;
    verify_light_token_balance(rpc, taker_token_a, token_a_offered, "taker_token_a (final)")
        .await;
    verify_light_token_balance(rpc, taker_token_b, 0, "taker_token_b (final)").await;

    let offer_after = rpc.get_account(offer_pda).await.unwrap();
    assert!(
        offer_after.is_none(),
        "Offer account should be closed after take_offer"
    );
}

// ============================================================================
// Instruction Helpers
// ============================================================================

/// Derive offer + vault PDAs, fetch validity proof, send `make_offer` instruction.
///
/// Returns `(offer_pda, vault_pda)`.
async fn make_offer<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &EscrowTestContext,
    maker_token_a: Pubkey,
    offer_id: u64,
    token_a_offered: u64,
    token_b_wanted: u64,
) -> (Pubkey, Pubkey) {
    let (offer_pda, _) = Pubkey::find_program_address(
        &[
            escrow::OFFER_SEED,
            ctx.maker.pubkey().as_ref(),
            offer_id.to_le_bytes().as_ref(),
        ],
        &ctx.program_id,
    );

    let (vault_pda, vault_bump) = Pubkey::find_program_address(
        &[escrow::VAULT_SEED, offer_pda.as_ref()],
        &ctx.program_id,
    );

    // Validity proof: verifies that the offer PDA's derived address does not
    // yet exist in the address tree. Required for Light Token account creation.
    let proof_result = get_create_accounts_proof(
        rpc,
        &ctx.program_id,
        vec![CreateAccountsProofInput::pda(offer_pda)],
    )
    .await
    .unwrap();

    let (_, spl_interface_bump_a) = find_spl_interface_pda(&ctx.mint_a_pubkey, false);

    let accounts = escrow::accounts::MakeOffer {
        fee_payer: ctx.maker.pubkey(),
        authority: ctx.authority_pda,
        compression_config: ctx.compression_config,
        token_mint_a: ctx.mint_a_pubkey,
        token_mint_b: ctx.mint_b_pubkey,
        maker_token_account_a: maker_token_a,
        offer: offer_pda,
        vault: vault_pda,
        token_program: ctx.token_config.token_program_id(),
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_config: COMPRESSIBLE_CONFIG_V1,
        pda_rent_sponsor: ctx.rent_sponsor,
        light_token_rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        spl_interface_pda_a: ctx
            .spl_interface_a
            .as_ref()
            .map(|i| i.pda)
            .unwrap_or_default(),
    };

    let data = escrow::instruction::MakeOffer {
        params: escrow::MakeOfferParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            id: offer_id,
            token_a_offered_amount: token_a_offered,
            token_b_wanted_amount: token_b_wanted,
            vault_bump,
            spl_interface_bump_a,
        },
    };

    let ix = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: data.data(),
    };

    rpc.create_and_send_transaction(&[ix], &ctx.maker.pubkey(), &[&ctx.maker])
        .await
        .expect("make_offer should succeed");

    (offer_pda, vault_pda)
}

/// Send `take_offer` instruction: taker sends token B to maker, vault releases token A to taker.
async fn take_offer<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    ctx: &EscrowTestContext,
    taker_token_a: Pubkey,
    taker_token_b: Pubkey,
    maker_token_b: Pubkey,
    offer_pda: Pubkey,
    vault_pda: Pubkey,
) {
    let (_, spl_interface_bump_a) = find_spl_interface_pda(&ctx.mint_a_pubkey, false);
    let (_, spl_interface_bump_b) = find_spl_interface_pda(&ctx.mint_b_pubkey, false);

    let accounts = escrow::accounts::TakeOffer {
        taker: ctx.taker.pubkey(),
        maker: ctx.maker.pubkey(),
        authority: ctx.authority_pda,
        token_mint_a: ctx.mint_a_pubkey,
        token_mint_b: ctx.mint_b_pubkey,
        taker_token_account_a: taker_token_a,
        taker_token_account_b: taker_token_b,
        maker_token_account_b: maker_token_b,
        offer: offer_pda,
        vault: vault_pda,
        token_program: ctx.token_config.token_program_id(),
        system_program: solana_sdk::system_program::ID,
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_cpi_authority: CPI_AUTHORITY_PDA,
        light_token_rent_sponsor: RENT_SPONSOR,
        spl_interface_pda_a: ctx
            .spl_interface_a
            .as_ref()
            .map(|i| i.pda)
            .unwrap_or_default(),
        spl_interface_pda_b: ctx
            .spl_interface_b
            .as_ref()
            .map(|i| i.pda)
            .unwrap_or_default(),
    };

    let data = escrow::instruction::TakeOffer {
        params: escrow::TakeOfferParams {
            spl_interface_bump_a,
            spl_interface_bump_b,
        },
    };

    let ix = Instruction {
        program_id: ctx.program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    rpc.create_and_send_transaction(&[ix], &ctx.taker.pubkey(), &[&ctx.taker])
        .await
        .expect("take_offer should succeed");
}
