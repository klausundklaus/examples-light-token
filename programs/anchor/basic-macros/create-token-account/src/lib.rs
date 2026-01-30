//! Create Light token account (vault) using `#[light_account(init, token, ...)]` macro.
//!
//! Demonstrates creating a compressed token account (vault) with the macro pattern.
//! The macro generates the CPI call to light-token program automatically.

#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk::derive_light_cpi_signer;
use light_sdk_macros::{light_program, LightAccounts};
use light_sdk_types::CpiSigner;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

declare_id!("9p5BUDtVmRRJqp2sN73ZUZDbaYtYvEWuxzrHH3A2ni9y");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("9p5BUDtVmRRJqp2sN73ZUZDbaYtYvEWuxzrHH3A2ni9y");

/// Seed for the vault authority PDA.
pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth";
/// Seed for the vault token account PDA.
pub const VAULT_SEED: &[u8] = b"vault";

/// Parameters for creating a token vault.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateTokenVaultParams {
    /// Proof for account creation.
    pub create_accounts_proof: CreateAccountsProof,
    /// Bump for the vault PDA (needed for invoke_signed).
    pub vault_bump: u8,
}

/// Accounts for creating a token vault.
///
/// The `#[light_account(init, token, ...)]` macro on `vault` generates the CPI
/// to create a compressed token account owned by `vault_authority`.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateTokenVaultParams)]
pub struct CreateTokenVault<'info> {
    /// Transaction fee payer.
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint for the vault.
    pub mint: AccountInfo<'info>,

    /// CHECK: Vault authority PDA - validated by seeds constraint.
    #[account(
        seeds = [VAULT_AUTH_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: Token vault account - validated by seeds constraint and light_account macro.
    /// The macro generates the CPI to create this as a compressed token account.
    #[account(
        mut,
        seeds = [VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    #[light_account(
        init,
        token::authority = [VAULT_SEED, self.mint.key()],
        token::mint = mint,
        token::owner = vault_authority,
        token::bump = params.vault_bump
    )]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: Validated by address constraint.
    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    /// CHECK: Validated by address constraint.
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority (required for token account creation).
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light token program for CPI.
    pub light_token_program: AccountInfo<'info>,

    /// System program.
    pub system_program: Program<'info, System>,
}

#[light_program]
#[program]
pub mod light_token_macro_create_token_account {
    use super::*;

    /// Create a token vault.
    ///
    /// The `#[light_account(init, token, ...)]` macro handles the actual token
    /// account creation via CPI to the light-token program.
    #[allow(unused_variables)]
    pub fn create_token_vault<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateTokenVault<'info>>,
        params: CreateTokenVaultParams,
    ) -> Result<()> {
        // Token vault creation is handled by the macro-generated LightFinalize implementation.
        Ok(())
    }
}
