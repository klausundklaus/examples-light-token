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

pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth";
pub const VAULT_SEED: &[u8] = b"vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateTokenVaultParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub vault_bump: u8,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateTokenVaultParams)]
pub struct CreateTokenVault<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint for the vault
    pub mint: AccountInfo<'info>,

    /// CHECK: Validated by seeds constraint
    #[account(
        seeds = [VAULT_AUTH_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: Validated by seeds constraint and light_account macro
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

    /// CHECK: Validated by address constraint
    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    /// CHECK: Validated by address constraint
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[light_program]
#[program]
pub mod light_token_macro_create_token_account {
    use super::*;

    #[allow(unused_variables)]
    pub fn create_token_vault<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateTokenVault<'info>>,
        params: CreateTokenVaultParams,
    ) -> Result<()> {
        Ok(())
    }
}
