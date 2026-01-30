#![allow(deprecated)]

pub mod constants;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda, CpiSigner};
use light_token::anchor::light_program;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("FKJs6rp6TXJtxzLiPtdYhqa9ExRuBXG2zwh4fda6WATN");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FKJs6rp6TXJtxzLiPtdYhqa9ExRuBXG2zwh4fda6WATN");

pub const PROGRAM_RENT_SPONSOR_DATA: ([u8; 32], u8) =
    derive_light_rent_sponsor_pda!("FKJs6rp6TXJtxzLiPtdYhqa9ExRuBXG2zwh4fda6WATN");

#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

#[light_program]
#[program]
pub mod escrow {
    use super::*;

    pub fn make_offer<'info>(
        mut ctx: Context<'_, '_, '_, 'info, MakeOffer<'info>>,
        params: MakeOfferParams,
    ) -> Result<()> {
        // Vault is created automatically by #[light_account(init, token::...)] macro
        instructions::make_offer::send_offered_tokens_to_vault(&ctx, &params)?;
        instructions::make_offer::save_offer(&mut ctx, &params)
    }

    pub fn take_offer(ctx: Context<TakeOffer>) -> Result<()> {
        instructions::take_offer::send_wanted_tokens_to_maker(&ctx)?;
        instructions::take_offer::withdraw_from_vault(&ctx)?;
        instructions::take_offer::close_vault(&ctx)
    }
}
