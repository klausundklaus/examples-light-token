#![allow(clippy::result_large_err)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda, CpiSigner};
use light_token::anchor::light_program;

mod constants;
mod errors;
mod instructions;
mod state;

// Re-export constants needed by #[light_program] macro
pub use constants::{AUTHORITY_SEED, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED};
// Re-export params structs for tests
pub use instructions::CreatePoolParams;

// Set the correct key here
declare_id!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

pub const PROGRAM_RENT_SPONSOR_DATA: ([u8; 32], u8) =
    derive_light_rent_sponsor_pda!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

#[light_program]
#[program]
pub mod swap_example {
    pub use super::instructions::*;
    use super::*;

    pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
        instructions::create_amm(ctx, id, fee)
    }

    pub fn create_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePool<'info>>,
        params: CreatePoolParams,
    ) -> Result<()> {
        instructions::create_pool(ctx, params)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        instructions::deposit_liquidity(ctx, amount_a, amount_b)
    }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
        instructions::withdraw_liquidity(ctx, amount)
    }

    pub fn swap_exact_tokens_for_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        instructions::swap_exact_tokens_for_tokens(ctx, swap_a, input_amount, min_output_amount)
    }
}
