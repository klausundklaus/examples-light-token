//! Fundraiser test setup for 5 token standard combinations: SPL, Token 2022, Light.
//!
//! Each combination varies the mint type and contributor account type while the vault
//! is always a Light Token account:
//!
//! - `Spl` / `Token2022`: standard associated token accounts, transfers via `TransferInterfaceCpi`
//! - `Light`: Light Token accounts, transfers via `TransferInterfaceCpi`
//! - `LightSpl` / `LightT22`: SPL/Token 2022 mints converted into Light Token accounts before
//!   the fundraiser starts (tokens are minted to a temporary associated token account, then
//!   transferred to an associated Light Token account via `transfer_spl_to_light`)
//!
//! ## Setup flow
//!
//! 1. `create_test_rpc()` — start test validator with fundraiser + minter programs
//! 2. `setup_fundraiser_test(config)` — create mint, interface PDAs, maker, derive PDAs
//! 3. `create_contributor()` — create funded contributor accounts per config

// ============================================================================
// Imports
// ============================================================================

use anchor_spl::token;
use shared_test_utils::{
    light_tokens::{create_light_ata, create_light_mint, mint_light_tokens},
    setup::initialize_rent_free_config,
    spl_interface::{create_spl_interface_pda, transfer_spl_to_light},
    spl_tokens::{create_spl_ata, create_spl_mint, mint_spl_tokens},
    t22_tokens::{create_t22_ata, create_t22_mint, mint_t22_tokens},
    Indexer, LightProgramTest, MintType, ProgramTestConfig, Rpc,
    SplInterfaceResult, TestRpc,
    LIGHT_TOKEN_MINTER_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ============================================================================
// Token Configuration
// ============================================================================

/// Token configuration for parameterized fundraiser tests.
///
/// Each variant determines the mint type and contributor account type. The vault is
/// always a Light Token account (rent-free). The contributor account type controls
/// which CPI path `transfer_tokens()` selects at runtime:
///
/// - SPL/Token 2022 contributor accounts → `TransferInterfaceCpi` (needs interface PDA)
/// - Light contributor accounts → `TransferInterfaceCpi` (no interface PDA)
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum TokenConfig {
    /// SPL mint + SPL associated token accounts and Light Token vault.
    Spl,
    /// Token 2022 mint + Token 2022 associated token accounts and Light Token vault.
    Token2022,
    /// SPL mint + associated Light Token accounts. Tokens converted from SPL associated token accounts in setup.
    /// During fundraiser, all transfers are Light-to-Light.
    LightSpl,
    /// Token 2022 mint + associated Light Token accounts. Tokens converted from Token 2022 associated token accounts in setup.
    /// During fundraiser, all transfers are Light-to-Light.
    LightT22,
    /// Light Token mint + associated Light Token accounts and Light Token vault.
    Light,
}

impl TokenConfig {
    /// Returns the underlying mint program type.
    /// Light mints use SPL-compatible layout, so this returns `MintType::Spl`.
    pub fn mint_type(&self) -> MintType {
        match self {
            TokenConfig::Spl | TokenConfig::LightSpl => MintType::Spl,
            TokenConfig::Token2022 | TokenConfig::LightT22 => MintType::Token2022,
            TokenConfig::Light => MintType::Spl, // Light mints use SPL-compatible layout
        }
    }

    /// Returns the token program ID passed as `token_program` in instructions.
    pub fn token_program_id(&self) -> Pubkey {
        match self {
            TokenConfig::Spl | TokenConfig::LightSpl => token::ID,
            TokenConfig::Token2022 | TokenConfig::LightT22 => spl_token_2022::ID,
            TokenConfig::Light => Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        }
    }

    /// Returns true if mints are Light Token mints (not SPL/Token 2022)
    pub fn uses_light_mints(&self) -> bool {
        matches!(self, TokenConfig::Light)
    }

}

// ============================================================================
// Test Context
// ============================================================================

/// Context for fundraiser tests containing all necessary accounts
#[allow(dead_code)]
pub struct FundraiserTestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub token_config: TokenConfig,
    pub compression_config: Pubkey,
    /// Per-program rent sponsor PDA (derived from program_id)
    pub rent_sponsor: Pubkey,

    // Mint
    pub mint_pubkey: Pubkey,
    pub light_mint_authority: Option<Keypair>,
    pub spl_interface: Option<SplInterfaceResult>,

    // Participants
    pub maker: Keypair,

    // PDAs
    pub fundraiser_pda: Pubkey,
    pub vault_pda: Pubkey,
    pub vault_bump: u8,
    pub authority_pda: Pubkey,

    // Fundraiser terms
    pub amount_to_raise: u64,
    pub duration: u16,
}

// ============================================================================
// Setup Functions
// ============================================================================

/// Create a new LightProgramTest instance for fundraiser tests
pub async fn create_test_rpc() -> LightProgramTest {
    let program_id = fundraiser::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![
            ("fundraiser", program_id),
            ("light_token_minter", LIGHT_TOKEN_MINTER_PROGRAM_ID),
        ]),
    );
    config = config.with_light_protocol_events();
    LightProgramTest::new(config).await.unwrap()
}

/// Initialize mint, interface PDAs, and participants for a given token config.
///
/// SPL interface PDAs are created for all SPL/T22 configs (including `Spl` and
/// `Token2022`, not just `LightSpl`/`LightT22`) because the vault is always a
/// Light Token account — `TransferInterfaceCpi` needs the interface PDA when an
/// SPL/Token 2022 account transfers to a Light Token vault.
///
/// For `Light` config, no interface PDA is created (early return).
///
/// Pass `duration_override` to set a custom fundraiser duration (in days).
/// Defaults to 7 days. Use `Some(1)` for refund tests to minimize warp time.
pub async fn setup_fundraiser_test<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    config: TokenConfig,
    duration_override: Option<u16>,
) -> FundraiserTestContext {
    let program_id = fundraiser::ID;
    let payer = rpc.get_payer().insecure_clone();

    let (compression_config, rent_sponsor) =
        initialize_rent_free_config(rpc, &payer, &program_id).await;

    let maker = Keypair::new();
    rpc.airdrop_lamports(&maker.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let amount_to_raise = 1_000_000_000_000u64; // 1000 tokens (9 decimals)
    let duration = duration_override.unwrap_or(7u16);

    let (authority_pda, _) = Pubkey::find_program_address(&[b"authority"], &program_id);

    let (fundraiser_pda, _) =
        Pubkey::find_program_address(&[b"fundraiser", maker.pubkey().as_ref()], &program_id);

    let (vault_pda, vault_bump) = Pubkey::find_program_address(
        &[fundraiser::VAULT_SEED, fundraiser_pda.as_ref()],
        &program_id,
    );

    if config.uses_light_mints() {
        // ========== LIGHT MINT SETUP ==========
        let light_mint = create_light_mint(
            rpc,
            &payer,
            9,
            "Fundraiser Token",
            "FUND",
            &compression_config,
        )
        .await;

        return FundraiserTestContext {
            program_id,
            payer,
            token_config: config,
            compression_config,
            rent_sponsor,
            mint_pubkey: light_mint.mint,
            light_mint_authority: Some(light_mint.authority),
            spl_interface: None,
            maker,
            fundraiser_pda,
            vault_pda,
            vault_bump,
            authority_pda,
            amount_to_raise,
            duration,
        };
    }

    // ========== SPL/T22 MINT SETUP ==========
    let mint = match config {
        TokenConfig::Spl | TokenConfig::LightSpl => {
            create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await
        }
        TokenConfig::Light => unreachable!("Light config handled above"),
    };

    let mint_pubkey = mint.pubkey();

    // Required for `TransferInterfaceCpi` between SPL/Token 2022 and Light Token vault.
    let iface =
        create_spl_interface_pda(rpc, &payer, &mint_pubkey, config.mint_type(), false).await;

    FundraiserTestContext {
        program_id,
        payer,
        token_config: config,
        compression_config,
        rent_sponsor,
        mint_pubkey,
        light_mint_authority: None,
        spl_interface: Some(iface),
        maker,
        fundraiser_pda,
        vault_pda,
        vault_bump,
        authority_pda,
        amount_to_raise,
        duration,
    }
}

// ============================================================================
// Contributor Account Creation
// ============================================================================

/// Create a contributor with a funded token account.
///
/// Account creation varies by config:
/// - `Spl` / `Token2022`: create standard associated token account, mint directly
/// - `Light`: create associated Light Token account via `mint_light_tokens` (creates + mints in one call)
/// - `LightSpl` / `LightT22`: create temporary SPL/Token 2022 associated token account → mint →
///   create associated Light Token account → convert via `transfer_spl_to_light`.
pub async fn create_contributor<R: Rpc + Indexer>(
    rpc: &mut R,
    ctx: &FundraiserTestContext,
    funding_amount: u64,
) -> (Keypair, Pubkey) {
    let contributor = Keypair::new();
    rpc.airdrop_lamports(&contributor.pubkey(), 5_000_000_000)
        .await
        .unwrap();

    let contributor_ata = match ctx.token_config {
        TokenConfig::Spl => {
            let ata =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &ata,
                &ctx.payer,
                funding_amount,
            )
            .await;
            ata
        }
        TokenConfig::Token2022 => {
            let ata =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &ata,
                &ctx.payer,
                funding_amount,
            )
            .await;
            ata
        }
        TokenConfig::LightSpl => {
            let temp_ata =
                create_spl_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_spl_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &temp_ata,
                &ctx.payer,
                funding_amount,
            )
            .await;

            let light_ata =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;

            let iface = ctx
                .spl_interface
                .as_ref()
                .expect("LightSpl requires SPL interface PDA");
            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &contributor,
                &ctx.mint_pubkey,
                9,
                &temp_ata,
                &light_ata,
                &iface.pda,
                iface.bump,
                funding_amount,
                MintType::Spl,
            )
            .await;

            light_ata
        }
        TokenConfig::LightT22 => {
            let temp_ata =
                create_t22_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;
            mint_t22_tokens(
                rpc,
                &ctx.payer,
                &ctx.mint_pubkey,
                &temp_ata,
                &ctx.payer,
                funding_amount,
            )
            .await;

            let light_ata =
                create_light_ata(rpc, &ctx.payer, &ctx.mint_pubkey, &contributor.pubkey()).await;

            let iface = ctx
                .spl_interface
                .as_ref()
                .expect("LightT22 requires SPL interface PDA");
            transfer_spl_to_light(
                rpc,
                &ctx.payer,
                &contributor,
                &ctx.mint_pubkey,
                9,
                &temp_ata,
                &light_ata,
                &iface.pda,
                iface.bump,
                funding_amount,
                MintType::Token2022,
            )
            .await;

            light_ata
        }
        TokenConfig::Light => {
            let mint_authority = ctx
                .light_mint_authority
                .as_ref()
                .expect("Light config requires mint authority");

            mint_light_tokens(
                rpc,
                &ctx.payer,
                mint_authority,
                &ctx.mint_pubkey,
                &contributor.pubkey(),
                funding_amount,
            )
            .await
        }
    };

    (contributor, contributor_ata)
}
