#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_token::anchor::{derive_light_cpi_signer, light_program, CpiSigner};

declare_id!("Eoiuq1dXvHxh6dLx3wh9gj8kSAUpga11krTrbfF5XYsC");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("Eoiuq1dXvHxh6dLx3wh9gj8kSAUpga11krTrbfF5XYsC");

mod constants;
mod error;
pub mod instructions;
mod state;

pub use constants::*;
use error::*;
pub use instructions::*;

#[light_program]
#[program]
pub mod fundraiser {
    use super::*;

    pub fn initialize<'info>(
        ctx: Context<'_, '_, '_, 'info, Initialize<'info>>,
        params: InitializeParams,
    ) -> Result<()> {
        ctx.accounts.initialize(&params, &ctx.bumps)
    }

    pub fn contribute(ctx: Context<Contribute>, amount: u64) -> Result<()> {
        ctx.accounts.contribute(amount)
    }

    pub fn check_contributions(ctx: Context<CheckContributions>) -> Result<()> {
        ctx.accounts.check_contributions(&ctx.bumps)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund(&ctx.bumps)
    }
}
