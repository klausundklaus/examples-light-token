#![allow(deprecated)]

pub mod constants;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;
use light_token::anchor::{derive_light_cpi_signer, light_program, CpiSigner};

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("FKJs6rp6TXJtxzLiPtdYhqa9ExRuBXG2zwh4fda6WATN");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FKJs6rp6TXJtxzLiPtdYhqa9ExRuBXG2zwh4fda6WATN");

#[light_program]
#[program]
pub mod escrow {
    use super::*;

    pub fn make_offer<'info>(
        mut ctx: Context<'_, '_, '_, 'info, MakeOffer<'info>>,
        params: MakeOfferParams,
    ) -> Result<()> {
        instructions::make_offer::create_vault(&ctx, &params)?;
        instructions::make_offer::send_offered_tokens_to_vault(&ctx, &params)?;
        instructions::make_offer::save_offer(&mut ctx, &params)
    }

    pub fn take_offer(ctx: Context<TakeOffer>) -> Result<()> {
        instructions::take_offer::send_wanted_tokens_to_maker(&ctx)?;
        instructions::take_offer::withdraw_from_vault(&ctx)?;
        instructions::take_offer::close_vault(&ctx)
    }
}
