#![allow(clippy::result_large_err, unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_account::{derive_light_cpi_signer, light_program, CpiSigner};

mod constants;
mod errors;
mod instructions;
mod state;

pub use constants::{AUTHORITY_SEED, LP_MINT_SIGNER_SEED, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED};
pub use instructions::{CreatePoolLightLpParams, CreatePoolParams};

declare_id!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

#[light_program]
#[allow(deprecated)] // Anchor's #[program] uses deprecated AccountInfo::realloc
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
        spl_interface_bump_a: u8,
        spl_interface_bump_b: u8,
    ) -> Result<()> {
        instructions::deposit_liquidity(ctx, amount_a, amount_b, spl_interface_bump_a, spl_interface_bump_b)
    }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64, spl_interface_bump_a: u8, spl_interface_bump_b: u8) -> Result<()> {
        instructions::withdraw_liquidity(ctx, amount, spl_interface_bump_a, spl_interface_bump_b)
    }

    pub fn swap_exact_tokens_for_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
        spl_interface_bump_a: u8,
        spl_interface_bump_b: u8,
    ) -> Result<()> {
        instructions::swap_exact_tokens_for_tokens(ctx, swap_a, input_amount, min_output_amount, spl_interface_bump_a, spl_interface_bump_b)
    }

    pub fn create_pool_light_lp<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePoolLightLp<'info>>,
        params: CreatePoolLightLpParams,
    ) -> Result<()> {
        instructions::create_pool_light_lp(ctx, params)
    }
}
