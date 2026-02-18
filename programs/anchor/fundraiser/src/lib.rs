#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_account::{derive_light_cpi_signer, light_program, CpiSigner};

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
// Anchor's #[program] macro uses deprecated AccountInfo::realloc internally
#[allow(deprecated)]
#[program]
pub mod fundraiser {
    use super::*;

    pub fn initialize<'info>(
        ctx: Context<'_, '_, '_, 'info, Initialize<'info>>,
        params: InitializeParams,
    ) -> Result<()> {
        ctx.accounts.initialize(&params, &ctx.bumps)
    }

    pub fn contribute(ctx: Context<Contribute>, amount: u64, spl_interface_bump: u8) -> Result<()> {
        ctx.accounts.contribute(amount, spl_interface_bump)
    }

    pub fn check_contributions(ctx: Context<CheckContributions>, spl_interface_bump: u8) -> Result<()> {
        ctx.accounts.check_contributions(&ctx.bumps, spl_interface_bump)
    }

    pub fn refund(ctx: Context<Refund>, spl_interface_bump: u8) -> Result<()> {
        ctx.accounts.refund(&ctx.bumps, spl_interface_bump)
    }
}
