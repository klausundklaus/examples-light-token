#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk::derive_light_cpi_signer;
use light_sdk_macros::{light_program, LightAccounts};
use light_sdk_types::{CpiSigner, LIGHT_TOKEN_PROGRAM_ID};
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

declare_id!("CLsn9MTFv97oMTsujRoQAw1u2rSm2HnKtGuWUbbc8Jfn");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("CLsn9MTFv97oMTsujRoQAw1u2rSm2HnKtGuWUbbc8Jfn");

#[light_program]
#[program]
pub mod light_token_macro_create_associated_token_account {
    use super::*;

    #[allow(unused_variables)]
    pub fn create_associated_token_account<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateAssociatedTokenAccount<'info>>,
        params: CreateAssociatedTokenAccountParams,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateAssociatedTokenAccountParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub associated_token_account_bump: u8,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateAssociatedTokenAccountParams)]
pub struct CreateAssociatedTokenAccount<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint for the associated token account
    pub associated_token_account_mint: AccountInfo<'info>,

    /// CHECK: Owner of the associated token account
    pub associated_token_account_owner: AccountInfo<'info>,

    /// CHECK: Validated by light_account macro
    #[account(mut)]
    #[light_account(init, associated_token::authority = associated_token_account_owner, associated_token::mint = associated_token_account_mint, associated_token::bump = params.associated_token_account_bump)]
    pub associated_token_account: UncheckedAccount<'info>,

    /// CHECK: Validated by address constraint
    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    /// CHECK: Validated by address constraint
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token program for CPI
    #[account(address = LIGHT_TOKEN_PROGRAM_ID.into())]
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
