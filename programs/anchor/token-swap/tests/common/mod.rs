//! AMM test setup for 6 token standard combinations: SPL, T22, Light.
//!
//! Each combination varies the mint type and user account type while the pool
//! vaults are always Light Token accounts:
//!
//! - `Spl` / `Token2022`: standard associated token accounts, transfers via `TransferInterfaceCpi`
//! - `Light` / `LightToLight`: Light Token accounts, transfers via `TransferInterfaceCpi`
//! - `LightSpl` / `LightT22`: SPL/Token 2022 mints converted into Light Token accounts before
//!   the AMM starts (tokens are minted to a temp associated token account, then
//!   transferred to an associated Light Token account via `transfer_spl_to_light`)
//!
//! `LightToLight` additionally creates the LP mint as a Light Token mint via `create_pool_light_lp`,
//! making the entire pool rent-free.
//!
//! ## Setup flow
//!
//! 1. `create_test_rpc()` — start test validator with swap_example + minter programs
//! 2. `setup_amm_test(config)` — create mints, interface PDAs, depositor, derive PDAs
//! 3. `create_trader()` — create funded trader accounts per config

// ============================================================================
// Imports
// ============================================================================

use anchor_spl::token;
use light_token::instruction::find_mint_address;
use shared_test_utils::{
    light_tokens::{create_light_ata, create_light_mint, mint_light_tokens},
    setup::initialize_rent_free_config,
    spl_interface::{create_spl_interface_pda, transfer_spl_to_light},
    spl_tokens::{create_spl_ata, create_spl_mint, mint_spl_tokens},
    t22_tokens::{create_t22_ata, create_t22_mint, mint_t22_tokens},
    Indexer, LightProgramTest, MintType, ProgramTestConfig, Rpc, TestRpc,
    LIGHT_TOKEN_MINTER_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022::pod::PodAccount;

// ============================================================================
// Token Configuration
// ============================================================================

/// Token configuration for parameterized AMM tests.
///
/// Each variant determines the mint type and user account type. The pool vaults
/// are always Light Token accounts (rent-free). The user account type controls
/// which CPI path `transfer_tokens()` selects at runtime:
///
/// - SPL/Token 2022 user accounts → `TransferInterfaceCpi` (needs interface PDA)
/// - Light user accounts → `TransferInterfaceCpi` (no interface PDA)
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum TokenConfig {
    /// SPL mint + SPL ATAs and Light Token vault. Transfers via `TransferInterfaceCpi`.
    Spl,
    /// Token 2022 mint + Token 2022 ATAs and Light Token vault. Transfers via `TransferInterfaceCpi`.
    Token2022,
    /// SPL mint + associated Light Token accounts. Tokens converted from SPL ATAs in setup.
    /// During AMM operations, all transfers are Light-to-Light.
    LightSpl,
    /// Token 2022 mint + associated Light Token accounts. Tokens converted from Token 2022 ATAs in setup.
    /// During AMM operations, all transfers are Light-to-Light.
    LightT22,
    /// Light Token mint + associated Light Token accounts + SPL LP mint.
    Light,
    /// Light Token mints + associated Light Token accounts + Light Token LP mint.
    LightToLight,
}

impl TokenConfig {
    /// Returns the underlying mint program type.
    /// Light Token mints use SPL-compatible layout, so this returns `MintType::Spl`.
    pub fn mint_type(&self) -> MintType {
        match self {
            TokenConfig::Spl | TokenConfig::LightSpl => MintType::Spl,
            TokenConfig::Token2022 | TokenConfig::LightT22 => MintType::Token2022,
            TokenConfig::Light | TokenConfig::LightToLight => MintType::Spl,
        }
    }

    /// Returns the token program ID passed as `token_program` in instructions.
    pub fn token_program_id(&self) -> Pubkey {
        match self {
            TokenConfig::Spl | TokenConfig::LightSpl => token::ID,
            TokenConfig::Token2022 | TokenConfig::LightT22 => spl_token_2022::ID,
            TokenConfig::Light | TokenConfig::LightToLight => {
                Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID)
            }
        }
    }

    /// Returns true if user accounts are Light Token accounts.
    pub fn uses_light_user_accounts(&self) -> bool {
        matches!(
            self,
            TokenConfig::LightSpl
                | TokenConfig::LightT22
                | TokenConfig::Light
                | TokenConfig::LightToLight
        )
    }

    /// Returns true if mints are Light Token mints (not SPL/Token 2022).
    pub fn uses_light_mints(&self) -> bool {
        matches!(self, TokenConfig::Light | TokenConfig::LightToLight)
    }

    /// Returns true if LP mint is a Light Token mint.
    pub fn uses_light_lp_mint(&self) -> bool {
        matches!(self, TokenConfig::LightToLight)
    }
}

// ============================================================================
// Test Context
// ============================================================================

/// Context for AMM tests containing all necessary accounts.
pub struct AmmTestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub mint_a_pubkey: Pubkey,
    pub mint_b_pubkey: Pubkey,
    pub amm_pda: Pubkey,
    pub amm_id: Pubkey,
    pub pool_pda: Pubkey,
    pub pool_authority: Pubkey,
    pub pool_authority_bump: u8,
    pub mint_liquidity: Pubkey,
    pub pool_account_a: Pubkey,
    pub pool_account_b: Pubkey,
    pub pool_a_bump: u8,
    pub pool_b_bump: u8,
    pub spl_interface_pda_a: Pubkey,
    pub spl_interface_pda_b: Pubkey,
    pub spl_interface_bump_a: u8,
    pub depositor: Keypair,
    pub depositor_ata_a: Pubkey,
    pub depositor_ata_b: Pubkey,
    pub token_config: TokenConfig,
    /// Authority keypair for Light Token mint A (if Light config).
    pub light_mint_authority_a: Option<Keypair>,
    /// LP mint signer PDA (for LightToLight config).
    pub lp_mint_signer: Option<Pubkey>,
    pub lp_mint_signer_bump: u8,
    /// Config PDA for rent-free Light Token mint creation.
    pub compression_config: Pubkey,
}

// ============================================================================
// Setup Functions
// ============================================================================

/// Create a new LightProgramTest instance for token-swap tests.
pub async fn create_test_rpc() -> LightProgramTest {
    let program_id = swap_example::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![
            ("swap_example", program_id),
            ("light_token_minter", LIGHT_TOKEN_MINTER_PROGRAM_ID),
        ]),
    );
    config = config.with_light_protocol_events();
    LightProgramTest::new(config).await.unwrap()
}

/// Initialize mints, interface PDAs, depositor, and derive PDAs for a given token config.
///
/// SPL interface PDAs are created for all SPL/Token 2022 configs (including `Spl` and
/// `Token2022`, not just `LightSpl`/`LightT22`) because the pool vaults are always
/// Light Token accounts — `TransferInterfaceCpi` needs the interface PDA when an
/// SPL/Token 2022 account transfers to a Light Token vault.
///
/// For `Light`/`LightToLight` configs, no interface PDA is created (early return).
pub async fn setup_amm_test<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    config: TokenConfig,
) -> AmmTestContext {
    let program_id = swap_example::ID;
    let payer = rpc.get_payer().insecure_clone();

    let (compression_config, _rent_sponsor) =
        initialize_rent_free_config(rpc, &payer, &program_id).await;

    let depositor = Keypair::new();
    rpc.airdrop_lamports(&depositor.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let amount = 10_000_000_000_000u64; // 10,000 tokens with 9 decimals

    if config.uses_light_mints() {
        // ========== LIGHT MINT SETUP ==========
        let light_mint_a =
            create_light_mint(rpc, &payer, 9, "Token A", "TOKA", &compression_config).await;
        let light_mint_b =
            create_light_mint(rpc, &payer, 9, "Token B", "TOKB", &compression_config).await;

        let mint_a_pubkey = light_mint_a.mint;
        let mint_b_pubkey = light_mint_b.mint;

        let spl_interface_pda_a = Pubkey::default();
        let spl_interface_pda_b = Pubkey::default();

        let depositor_ata_a = mint_light_tokens(
            rpc,
            &payer,
            &light_mint_a.authority,
            &mint_a_pubkey,
            &depositor.pubkey(),
            amount,
        )
        .await;
        let depositor_ata_b = mint_light_tokens(
            rpc,
            &payer,
            &light_mint_b.authority,
            &mint_b_pubkey,
            &depositor.pubkey(),
            amount,
        )
        .await;

        let amm_id = Pubkey::new_unique();
        let (amm_pda, _) = Pubkey::find_program_address(&[amm_id.as_ref()], &program_id);

        let (pool_pda, _) = Pubkey::find_program_address(
            &[
                amm_pda.as_ref(),
                mint_a_pubkey.as_ref(),
                mint_b_pubkey.as_ref(),
            ],
            &program_id,
        );

        let (pool_authority, pool_authority_bump) =
            Pubkey::find_program_address(&[b"authority"], &program_id);

        let (pool_account_a, pool_a_bump) =
            Pubkey::find_program_address(&[b"pool_a", pool_pda.as_ref()], &program_id);
        let (pool_account_b, pool_b_bump) =
            Pubkey::find_program_address(&[b"pool_b", pool_pda.as_ref()], &program_id);

        // Light config: SPL LP mint via standard PDA.
        // LightToLight config: Light Token LP mint via lp_mint_signer PDA.
        let (mint_liquidity, lp_mint_signer, lp_mint_signer_bump) =
            if config.uses_light_lp_mint() {
                let (lp_mint_signer, lp_mint_signer_bump) = Pubkey::find_program_address(
                    &[b"lp_mint_signer", pool_pda.as_ref()],
                    &program_id,
                );
                let (light_lp_mint, _) = find_mint_address(&lp_mint_signer);
                (light_lp_mint, Some(lp_mint_signer), lp_mint_signer_bump)
            } else {
                let (mint_liquidity, _) = Pubkey::find_program_address(
                    &[
                        amm_pda.as_ref(),
                        mint_a_pubkey.as_ref(),
                        mint_b_pubkey.as_ref(),
                        b"liquidity",
                    ],
                    &program_id,
                );
                (mint_liquidity, None, 0)
            };

        return AmmTestContext {
            program_id,
            payer,
            mint_a_pubkey,
            mint_b_pubkey,
            amm_pda,
            amm_id,
            pool_pda,
            pool_authority,
            pool_authority_bump,
            mint_liquidity,
            pool_account_a,
            pool_account_b,
            pool_a_bump,
            pool_b_bump,
            spl_interface_pda_a,
            spl_interface_pda_b,
            spl_interface_bump_a: 0,
            depositor,
            depositor_ata_a,
            depositor_ata_b,
            token_config: config,
            light_mint_authority_a: Some(light_mint_a.authority),
            lp_mint_signer,
            lp_mint_signer_bump,
            compression_config,
        };
    }

    // ========== SPL/T22 MINT SETUP ==========
    let (mint_a, mint_b) = match config {
        TokenConfig::Spl | TokenConfig::LightSpl => {
            let mint_a = create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await;
            let mint_b = create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await;
            (mint_a, mint_b)
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            let mint_a = create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await;
            let mint_b = create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await;
            (mint_a, mint_b)
        }
        TokenConfig::Light | TokenConfig::LightToLight => {
            unreachable!("Light/LightToLight config handled above")
        }
    };

    let mint_a_pubkey = mint_a.pubkey();
    let mint_b_pubkey = mint_b.pubkey();

    // Required for `TransferInterfaceCpi` between SPL/Token 2022 accounts and Light Token pool vaults.
    let spl_interface_result_a =
        create_spl_interface_pda(rpc, &payer, &mint_a_pubkey, config.mint_type(), false).await;
    let spl_interface_result_b =
        create_spl_interface_pda(rpc, &payer, &mint_b_pubkey, config.mint_type(), false).await;

    let (depositor_ata_a, depositor_ata_b) = match config {
        TokenConfig::Spl => {
            let ata_a = create_spl_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let ata_b = create_spl_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_spl_tokens(rpc, &payer, &mint_a_pubkey, &ata_a, &payer, amount).await;
            mint_spl_tokens(rpc, &payer, &mint_b_pubkey, &ata_b, &payer, amount).await;
            (ata_a, ata_b)
        }
        TokenConfig::Token2022 => {
            let ata_a = create_t22_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let ata_b = create_t22_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_t22_tokens(rpc, &payer, &mint_a_pubkey, &ata_a, &payer, amount).await;
            mint_t22_tokens(rpc, &payer, &mint_b_pubkey, &ata_b, &payer, amount).await;
            (ata_a, ata_b)
        }
        TokenConfig::LightSpl => {
            // Tokens route through temp SPL ATAs because Light Token accounts
            // cannot be direct mint targets for SPL mints.
            let temp_ata_a =
                create_spl_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let temp_ata_b =
                create_spl_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_spl_tokens(rpc, &payer, &mint_a_pubkey, &temp_ata_a, &payer, amount).await;
            mint_spl_tokens(rpc, &payer, &mint_b_pubkey, &temp_ata_b, &payer, amount).await;

            let light_ata_a =
                create_light_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;

            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &spl_interface_result_a.pda,
                spl_interface_result_a.bump,
                amount,
                MintType::Spl,
            )
            .await;
            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_b_pubkey,
                9,
                &temp_ata_b,
                &light_ata_b,
                &spl_interface_result_b.pda,
                spl_interface_result_b.bump,
                amount,
                MintType::Spl,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::LightT22 => {
            // Tokens route through temp Token 2022 ATAs because Light Token accounts
            // cannot be direct mint targets for Token 2022 mints.
            let temp_ata_a =
                create_t22_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let temp_ata_b =
                create_t22_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;
            mint_t22_tokens(rpc, &payer, &mint_a_pubkey, &temp_ata_a, &payer, amount).await;
            mint_t22_tokens(rpc, &payer, &mint_b_pubkey, &temp_ata_b, &payer, amount).await;

            let light_ata_a =
                create_light_ata(rpc, &payer, &mint_a_pubkey, &depositor.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &payer, &mint_b_pubkey, &depositor.pubkey()).await;

            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &spl_interface_result_a.pda,
                spl_interface_result_a.bump,
                amount,
                MintType::Token2022,
            )
            .await;
            transfer_spl_to_light(
                rpc,
                &payer,
                &depositor,
                &mint_b_pubkey,
                9,
                &temp_ata_b,
                &light_ata_b,
                &spl_interface_result_b.pda,
                spl_interface_result_b.bump,
                amount,
                MintType::Token2022,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::Light | TokenConfig::LightToLight => {
            unreachable!("Light/LightToLight config handled above")
        }
    };

    let amm_id = Pubkey::new_unique();
    let (amm_pda, _) = Pubkey::find_program_address(&[amm_id.as_ref()], &program_id);

    let (pool_pda, _) = Pubkey::find_program_address(
        &[
            amm_pda.as_ref(),
            mint_a_pubkey.as_ref(),
            mint_b_pubkey.as_ref(),
        ],
        &program_id,
    );

    let (pool_authority, pool_authority_bump) =
        Pubkey::find_program_address(&[b"authority"], &program_id);

    let (mint_liquidity, _) = Pubkey::find_program_address(
        &[
            amm_pda.as_ref(),
            mint_a_pubkey.as_ref(),
            mint_b_pubkey.as_ref(),
            b"liquidity",
        ],
        &program_id,
    );

    let (pool_account_a, pool_a_bump) =
        Pubkey::find_program_address(&[b"pool_a", pool_pda.as_ref()], &program_id);
    let (pool_account_b, pool_b_bump) =
        Pubkey::find_program_address(&[b"pool_b", pool_pda.as_ref()], &program_id);

    AmmTestContext {
        program_id,
        payer,
        mint_a_pubkey,
        mint_b_pubkey,
        amm_pda,
        amm_id,
        pool_pda,
        pool_authority,
        pool_authority_bump,
        mint_liquidity,
        pool_account_a,
        pool_account_b,
        pool_a_bump,
        pool_b_bump,
        spl_interface_pda_a: spl_interface_result_a.pda,
        spl_interface_pda_b: spl_interface_result_b.pda,
        spl_interface_bump_a: spl_interface_result_a.bump,
        depositor,
        depositor_ata_a,
        depositor_ata_b,
        token_config: config,
        light_mint_authority_a: None,
        lp_mint_signer: None,
        lp_mint_signer_bump: 0,
        compression_config,
    }
}

// ============================================================================
// Account Creation
// ============================================================================

/// Create a trader with funded token accounts.
///
/// Account creation varies by config:
/// - `Spl` / `Token2022`: create standard ATAs, mint directly
/// - `Light` / `LightToLight`: create associated Light Token accounts via `mint_light_tokens` (creates + mints in one call)
/// - `LightSpl` / `LightT22`: create temp SPL/Token 2022 ATAs, mint, then
///   create associated Light Token accounts and convert via `transfer_spl_to_light`
pub async fn create_trader<R: Rpc + Indexer>(
    rpc: &mut R,
    ctx: &AmmTestContext,
    initial_a: u64,
) -> (Keypair, Pubkey, Pubkey) {
    let trader = Keypair::new();
    rpc.airdrop_lamports(&trader.pubkey(), 5_000_000_000)
        .await
        .unwrap();

    let (trader_ata_a, trader_ata_b) = match ctx.token_config {
        TokenConfig::Spl => {
            let ata_a =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let ata_b =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;
            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;
            (ata_a, ata_b)
        }
        TokenConfig::Token2022 => {
            let ata_a =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let ata_b =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;
            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;
            (ata_a, ata_b)
        }
        TokenConfig::LightSpl => {
            // Tokens route through temp SPL ATAs because Light Token accounts
            // cannot be direct mint targets for SPL mints.
            let temp_ata_a =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let _temp_ata_b =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &temp_ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;

            let light_ata_a =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &trader,
                &ctx.mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &ctx.spl_interface_pda_a,
                ctx.spl_interface_bump_a,
                initial_a,
                MintType::Spl,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::LightT22 => {
            // Tokens route through temp Token 2022 ATAs because Light Token accounts
            // cannot be direct mint targets for Token 2022 mints.
            let temp_ata_a =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let _temp_ata_b =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_a_pubkey,
                &temp_ata_a,
                &ctx.payer,
                initial_a,
            )
            .await;

            let light_ata_a =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_a_pubkey, &trader.pubkey()).await;
            let light_ata_b =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &trader,
                &ctx.mint_a_pubkey,
                9,
                &temp_ata_a,
                &light_ata_a,
                &ctx.spl_interface_pda_a,
                ctx.spl_interface_bump_a,
                initial_a,
                MintType::Token2022,
            )
            .await;

            (light_ata_a, light_ata_b)
        }
        TokenConfig::Light | TokenConfig::LightToLight => {
            // Light Token mints support direct minting to associated Light Token accounts.
            let mint_authority_a = ctx
                .light_mint_authority_a
                .as_ref()
                .expect("Light/LightToLight config should have mint authority A");

            let light_ata_a = mint_light_tokens(
                rpc,
                &ctx.payer,
                mint_authority_a,
                &ctx.mint_a_pubkey,
                &trader.pubkey(),
                initial_a,
            )
            .await;

            // Token B: trader starts with zero balance, only needs the account created.
            let light_ata_b =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_b_pubkey, &trader.pubkey()).await;

            (light_ata_a, light_ata_b)
        }
    };

    (trader, trader_ata_a, trader_ata_b)
}

// ============================================================================
// Helpers
// ============================================================================

/// Get token balance from an active token account (SPL/Token 2022/Light Token).
pub async fn get_token_balance<R: Rpc>(rpc: &mut R, account: Pubkey) -> u64 {
    let account_data = rpc
        .get_account(account)
        .await
        .unwrap()
        .expect("Token account should exist");

    let token_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&account_data.data[..165]).unwrap();
    u64::from(token_state.amount)
}
