//! Escrow test setup for 5 token standard combinations: SPL, T22, Light.
//!
//! Each combination varies the mint type and associated token account type while the vault
//! is always a Light Token account:
//!
//! - `Spl` / `Token2022`: standard associated token accounts, transfers via `TransferInterfaceCpi`
//! - `Light`: Light Token accounts, transfers via `TransferInterfaceCpi`
//! - `LightSpl` / `LightT22`: SPL/Token 2022 mints converted into Light Token accounts before
//!   the escrow starts (tokens are minted to a temp associated token account, then
//!   transferred to an associated Light Token account via `transfer_spl_to_light`)
//!
//! ## Setup flow
//!
//! 1. `create_test_rpc()` — start test validator with escrow + minter programs
//! 2. `setup_escrow_test(config)` — create mints, interface PDAs, maker/taker
//! 3. `create_token_account()` — create funded/unfunded accounts per config

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
use anchor_lang::AnchorDeserialize;
use escrow::escrow::{LightAccountVariant, OfferSeeds, VaultSeeds};
use light_account::token::{Token as LightToken, TokenDataWithSeeds};
use light_account::IntoVariant;
use light_client::interface::{
    create_load_instructions, AccountInterface, AccountSpec, PdaSpec,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// ============================================================================
// Token Configuration
// ============================================================================

/// Token configuration for parameterized escrow tests.
///
/// Each variant determines the mint type and user account type. The vault is
/// always a Light Token account (rent-free). The user account type controls
/// which CPI path `TransferInterfaceCpi` selects at runtime:
///
/// - SPL/Token 2022 user accounts → cross-standard transfer (needs interface PDA)
/// - Light user accounts → light-to-light transfer (no interface PDA)
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum TokenConfig {
    /// SPL mint + SPL associated token accounts and Light Token vault.
    Spl,
    /// Token 2022 mint + Token 2022 associated token accounts and Light Token vault.
    Token2022,
    /// SPL mint + associated Light Token accounts. Tokens converted from SPL associated token accounts in setup.
    LightSpl,
    /// Token 2022 mint + associated Light Token accounts. Tokens converted from Token 2022 associated token accounts in setup.
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

/// Context for escrow tests containing all necessary accounts
#[allow(dead_code)]
pub struct EscrowTestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub token_config: TokenConfig,
    pub compression_config: Pubkey,
    /// Per-program rent sponsor PDA (derived from program_id)
    pub rent_sponsor: Pubkey,

    // Mint A
    pub mint_a_pubkey: Pubkey,
    pub light_mint_a_authority: Option<Keypair>,
    pub spl_interface_a: Option<SplInterfaceResult>,

    // Mint B
    pub mint_b_pubkey: Pubkey,
    pub light_mint_b_authority: Option<Keypair>,
    pub spl_interface_b: Option<SplInterfaceResult>,

    // Participants
    pub maker: Keypair,
    pub taker: Keypair,

    // PDAs
    pub authority_pda: Pubkey,
}

// ============================================================================
// Setup Functions
// ============================================================================

/// Create a new LightProgramTest instance for escrow tests
pub async fn create_test_rpc() -> LightProgramTest {
    let program_id = escrow::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![
            ("escrow", program_id),
            ("light_token_minter", LIGHT_TOKEN_MINTER_PROGRAM_ID),
        ]),
    );
    config = config.with_light_protocol_events();
    LightProgramTest::new(config).await.unwrap()
}

/// Initialize mints, interface PDAs, and participants for a given token config.
///
/// SPL interface PDAs are created for all SPL/Token 2022 configs (including `Spl` and
/// `Token2022`, not just `LightSpl`/`LightT22`) because the vault is always a
/// Light Token account — `TransferInterfaceCpi` needs the interface PDA when an
/// SPL/Token 2022 account transfers to a Light Token vault.
///
/// For `Light` config, no interface PDAs are created (early return).
pub async fn setup_escrow_test<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    config: TokenConfig,
) -> EscrowTestContext {
    let program_id = escrow::ID;
    let payer = rpc.get_payer().insecure_clone();

    let (compression_config, rent_sponsor) = initialize_rent_free_config(rpc, &payer, &program_id).await;

    let maker = Keypair::new();
    let taker = Keypair::new();
    rpc.airdrop_lamports(&maker.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(&taker.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let (authority_pda, _) =
        Pubkey::find_program_address(&[escrow::AUTH_SEED], &program_id);

    if config.uses_light_mints() {
        let light_mint_a = create_light_mint(
            rpc,
            &payer,
            9,
            "Escrow Token A",
            "ETKA",
            &compression_config,
        )
        .await;

        let light_mint_b = create_light_mint(
            rpc,
            &payer,
            9,
            "Escrow Token B",
            "ETKB",
            &compression_config,
        )
        .await;

        return EscrowTestContext {
            program_id,
            payer,
            token_config: config,
            compression_config,
            rent_sponsor,
            mint_a_pubkey: light_mint_a.mint,
            light_mint_a_authority: Some(light_mint_a.authority),
            spl_interface_a: None,
            mint_b_pubkey: light_mint_b.mint,
            light_mint_b_authority: Some(light_mint_b.authority),
            spl_interface_b: None,
            maker,
            taker,
            authority_pda,
        };
    }

    // ========== SPL/TOKEN 2022 MINT SETUP ==========
    let (mint_a, mint_b) = match config {
        TokenConfig::Spl | TokenConfig::LightSpl => {
            let a = create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await;
            let b = create_spl_mint(rpc, &payer, &payer.pubkey(), 9).await;
            (a, b)
        }
        TokenConfig::Token2022 | TokenConfig::LightT22 => {
            let a = create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await;
            let b = create_t22_mint(rpc, &payer, &payer.pubkey(), 9).await;
            (a, b)
        }
        TokenConfig::Light => unreachable!("Light config handled above"),
    };

    let mint_a_pubkey = mint_a.pubkey();
    let mint_b_pubkey = mint_b.pubkey();

    // Required for `TransferInterfaceCpi` between SPL/Token 2022 accounts and Light Token vault.
    let iface_a = create_spl_interface_pda(
        rpc,
        &payer,
        &mint_a_pubkey,
        config.mint_type(),
        false,
    )
    .await;
    let iface_b = create_spl_interface_pda(
        rpc,
        &payer,
        &mint_b_pubkey,
        config.mint_type(),
        false,
    )
    .await;
    let (spl_interface_a, spl_interface_b) = (Some(iface_a), Some(iface_b));

    EscrowTestContext {
        program_id,
        payer,
        token_config: config,
        compression_config,
        rent_sponsor,
        mint_a_pubkey,
        light_mint_a_authority: None,
        spl_interface_a,
        mint_b_pubkey,
        light_mint_b_authority: None,
        spl_interface_b,
        maker,
        taker,
        authority_pda,
    }
}

// ============================================================================
// Token Account Creation
// ============================================================================

/// Create a token account for a participant, optionally funded.
///
/// Account creation varies by config:
/// - `Spl` / `Token2022`: create standard associated token account, mint directly
/// - `Light`: create associated Light Token account via `mint_light_tokens` (creates + mints in one call)
/// - `LightSpl` / `LightT22`: create temporary SPL/Token 2022 associated token account → mint →
///   create associated Light Token account → convert via `transfer_spl_to_light`.
///   If unfunded, just creates the associated Light Token account.
pub async fn create_token_account<R: Rpc + Indexer>(
    rpc: &mut R,
    ctx: &EscrowTestContext,
    owner: &Keypair,
    mint_pubkey: &Pubkey,
    light_mint_authority: Option<&Keypair>,
    spl_interface: Option<&SplInterfaceResult>,
    funding_amount: u64,
) -> Pubkey {
    match ctx.token_config {
        TokenConfig::Spl => {
            let ata = create_spl_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await;
            if funding_amount > 0 {
                mint_spl_tokens(rpc, &ctx.payer, mint_pubkey, &ata, &ctx.payer, funding_amount)
                    .await;
            }
            ata
        }
        TokenConfig::Token2022 => {
            let ata = create_t22_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await;
            if funding_amount > 0 {
                mint_t22_tokens(rpc, &ctx.payer, mint_pubkey, &ata, &ctx.payer, funding_amount)
                    .await;
            }
            ata
        }
        TokenConfig::LightSpl => {
            if funding_amount > 0 {
                let temp_ata =
                    create_spl_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await;
                mint_spl_tokens(
                    rpc,
                    &ctx.payer,
                    mint_pubkey,
                    &temp_ata,
                    &ctx.payer,
                    funding_amount,
                )
                .await;

                let light_ata =
                    create_light_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await;

                let iface = spl_interface.expect("LightSpl requires SPL interface PDA");
                transfer_spl_to_light(
                    rpc,
                    &ctx.payer,
                    owner,
                    mint_pubkey,
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
            } else {
                create_light_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await
            }
        }
        TokenConfig::LightT22 => {
            if funding_amount > 0 {
                let temp_ata =
                    create_t22_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await;
                mint_t22_tokens(
                    rpc,
                    &ctx.payer,
                    mint_pubkey,
                    &temp_ata,
                    &ctx.payer,
                    funding_amount,
                )
                .await;

                let light_ata =
                    create_light_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await;

                let iface = spl_interface.expect("LightT22 requires SPL interface PDA");
                transfer_spl_to_light(
                    rpc,
                    &ctx.payer,
                    owner,
                    mint_pubkey,
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
            } else {
                create_light_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await
            }
        }
        TokenConfig::Light => {
            let mint_authority =
                light_mint_authority.expect("Light config requires mint authority");
            if funding_amount > 0 {
                mint_light_tokens(
                    rpc,
                    &ctx.payer,
                    mint_authority,
                    mint_pubkey,
                    &owner.pubkey(),
                    funding_amount,
                )
                .await
            } else {
                create_light_ata(rpc, &ctx.payer, mint_pubkey, &owner.pubkey()).await
            }
        }
    }
}

// ============================================================================
// Cold/Hot Lifecycle
// ============================================================================

/// Simulate the cold/hot lifecycle by advancing slots past sponsored rent.
///
/// Light Token accounts may turn cold between transactions. The Light Token
/// Program sponsors rent-exemption; when an account's virtual rent balance
/// drops below threshold, it auto-compresses: account data moves to a
/// state tree and on-chain lookups return `is_initialized: false`.
/// After compressing, the test loads cold accounts back to active state
/// before the next transaction.
/// Standard SPL/Token 2022 accounts are unaffected.
pub async fn warp_to_compress<R: Rpc + TestRpc + Indexer>(rpc: &mut R) {
    rpc.warp_epoch_forward(30)
        .await
        .expect("warp_epoch_forward should succeed");
}

/// Load all accounts referenced by the `make_offer` instruction.
///
/// Loads cold mints A/B and maker's ATA for token A back to active state.
/// No-op for accounts that are already hot or non-Light (SPL/Token 2022).
pub async fn load_accounts_make_offer<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    payer: &Keypair,
    maker: &Keypair,
    compression_config: Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) {
    let mut specs: Vec<AccountSpec<LightAccountVariant>> = Vec::new();

    // Mints
    for mint in [mint_a, mint_b] {
        if let Ok(response) = rpc.get_mint_interface(mint, None).await {
            if let Some(iface) = response.value {
                if iface.is_cold() {
                    specs.push(AccountSpec::Mint(AccountInterface::from(iface)));
                }
            }
        }
    }

    // Maker's ATA for token A
    if let Ok(response) = rpc.get_associated_token_account_interface(&maker.pubkey(), mint_a, None).await {
        if let Some(iface) = response.value {
            if iface.is_cold() {
                specs.push(AccountSpec::Ata(Box::new(iface)));
            }
        }
    }

    if specs.is_empty() {
        return;
    }

    let ixs = create_load_instructions::<LightAccountVariant, _>(
        &specs,
        payer.pubkey(),
        compression_config,
        &*rpc,
    )
    .await
    .expect("create_load_instructions for make_offer should succeed");

    if !ixs.is_empty() {
        rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[payer, maker])
            .await
            .expect("load for make_offer should succeed");
    }
}

/// Load all accounts referenced by the `take_offer` instruction.
///
/// Loads cold mints A/B, offer PDA, vault PDA, taker's ATAs for A/B,
/// and maker's ATA for B back to active state.
/// No-op for accounts that are already hot or non-Light (SPL/Token 2022).
pub async fn load_accounts_take_offer<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    payer: &Keypair,
    maker: &Keypair,
    taker: &Keypair,
    program_id: &Pubkey,
    compression_config: Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    offer_pda: Pubkey,
    vault_pda: Pubkey,
) {
    let mut specs: Vec<AccountSpec<LightAccountVariant>> = Vec::new();
    let mut needs_taker_signer = false;
    let mut needs_maker_signer = false;

    // Mints
    for mint in [mint_a, mint_b] {
        if let Ok(response) = rpc.get_mint_interface(mint, None).await {
            if let Some(iface) = response.value {
                if iface.is_cold() {
                    specs.push(AccountSpec::Mint(AccountInterface::from(iface)));
                }
            }
        }
    }

    // Offer PDA
    match rpc.get_account_interface(&offer_pda, None).await {
        Ok(response) => {
            if let Some(iface) = response.value {
                if iface.is_cold() {
                    let data = iface.data();
                    let offer: escrow::Offer =
                        AnchorDeserialize::deserialize(&mut &data[8..])
                            .expect("deserialize Offer from cold data");
                    let maker_pubkey = offer.maker;
                    let variant = OfferSeeds {
                        fee_payer: maker_pubkey,
                        id: offer.id,
                    }
                    .into_variant(&iface.data()[8..])
                    .expect("seed verification should pass");
                    specs.push(AccountSpec::Pda(PdaSpec::new(iface, variant, *program_id)));
                }
            }
        }
        _ => {}
    }

    // Vault PDA
    match rpc.get_token_account_interface(&vault_pda, None).await {
        Ok(response) => {
            if let Some(iface) = response.value {
                if iface.is_cold() {
                    let token_data: LightToken =
                        AnchorDeserialize::deserialize(&mut &iface.account.data[..])
                            .expect("deserialize Token from cold vault data");
                    let vault_variant =
                        LightAccountVariant::Vault(TokenDataWithSeeds {
                            seeds: VaultSeeds { offer: offer_pda },
                            token_data,
                        });
                    specs.push(AccountSpec::Pda(PdaSpec::new(
                        AccountInterface::from(iface),
                        vault_variant,
                        *program_id,
                    )));
                }
            }
        }
        _ => {}
    }

    // Taker's ATAs for A and B
    for mint in [mint_a, mint_b] {
        if let Ok(response) = rpc.get_associated_token_account_interface(&taker.pubkey(), mint, None).await {
            if let Some(iface) = response.value {
                if iface.is_cold() {
                    specs.push(AccountSpec::Ata(Box::new(iface)));
                    needs_taker_signer = true;
                }
            }
        }
    }

    // Maker's ATA for B (receives taker's payment)
    if let Ok(response) = rpc.get_associated_token_account_interface(&maker.pubkey(), mint_b, None).await {
        if let Some(iface) = response.value {
            if iface.is_cold() {
                specs.push(AccountSpec::Ata(Box::new(iface)));
                needs_maker_signer = true;
            }
        }
    }

    if specs.is_empty() {
        return;
    }

    let ixs = create_load_instructions::<LightAccountVariant, _>(
        &specs,
        payer.pubkey(),
        compression_config,
        &*rpc,
    )
    .await
    .expect("create_load_instructions for take_offer should succeed");

    if !ixs.is_empty() {
        // Only include signers whose accounts are actually being loaded.
        // ATA loading requires the wallet owner to sign; PDA/mint loading only needs the payer.
        let mut signers: Vec<&Keypair> = vec![payer];
        if needs_maker_signer {
            signers.push(maker);
        }
        if needs_taker_signer {
            signers.push(taker);
        }

        rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &signers)
            .await
            .expect("load for take_offer should succeed");
    }
}
