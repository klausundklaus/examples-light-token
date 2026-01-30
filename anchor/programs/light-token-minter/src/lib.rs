#![allow(deprecated)]

pub mod instructions;

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda, CpiSigner};
use light_token::anchor::light_program;

pub use instructions::*;

declare_id!("3EPJBoxM8Evtv3Wk7R2mSWsrSzUD7WSKAaYugLgpCitV");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("3EPJBoxM8Evtv3Wk7R2mSWsrSzUD7WSKAaYugLgpCitV");

pub const PROGRAM_RENT_SPONSOR_DATA: ([u8; 32], u8) =
    derive_light_rent_sponsor_pda!("3EPJBoxM8Evtv3Wk7R2mSWsrSzUD7WSKAaYugLgpCitV");

#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

/// Seed for deriving the mint signer PDA
pub const MINT_SIGNER_SEED: &[u8] = b"mint_signer";

#[light_program]
#[program]
pub mod light_token_minter {
    use super::*;

    pub fn create_mint<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateMint<'info>>,
        params: CreateMintParams,
    ) -> Result<()> {
        instructions::create::create_token(&ctx, &params)
    }

    pub fn mint_to<'info>(
        ctx: Context<'_, '_, '_, 'info, MintTo<'info>>,
        params: MintTokenParams,
    ) -> Result<()> {
        instructions::mint::mint_token(&ctx, &params)
    }
}
