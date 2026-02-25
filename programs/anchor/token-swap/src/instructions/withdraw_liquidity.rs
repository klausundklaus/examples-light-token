use anchor_lang::prelude::*;
use fixed::types::I64F64;
use light_anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};
use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{BurnCpi, TransferInterfaceCpi, LIGHT_TOKEN_RENT_SPONSOR};
use light_token::utils::get_token_account_balance;

use crate::{
    constants::{AUTHORITY_SEED, MINIMUM_LIQUIDITY, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED},
    errors::SwapError,
    state::{Amm, Pool},
};

pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64, spl_interface_bump_a: u8, spl_interface_bump_b: u8) -> Result<()> {
    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        &[authority_bump],
    ];

    let is_light_lp =
        ctx.accounts.liquidity_token_program.key().to_bytes() == LIGHT_TOKEN_PROGRAM_ID;

    let lp_supply = ctx.accounts.pool.lp_supply;

    let pool_a_balance =
        get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())?;
    let pool_b_balance =
        get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())?;

    let amount_a = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(pool_a_balance))
        .ok_or(SwapError::Overflow)?
        .checked_div(I64F64::from_num(lp_supply + MINIMUM_LIQUIDITY))
        .ok_or(SwapError::Underflow)?
        .floor()
        .to_num::<u64>();

    let decimals_a = ctx.accounts.mint_a.decimals;

    let mut cpi_a = TransferInterfaceCpi::new(
        amount_a,
        decimals_a,
        ctx.accounts.pool_account_a.to_account_info(),
        ctx.accounts.depositor_account_a.to_account_info(),
        ctx.accounts.pool_authority.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.mint_a.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    );
    if ctx.accounts.spl_interface_pda_a.is_some() {
        cpi_a = cpi_a.with_spl_interface(
            Some(ctx.accounts.mint_a.to_account_info()),
            Some(ctx.accounts.token_program.to_account_info()),
            ctx.accounts.spl_interface_pda_a.as_ref().map(|a| a.to_account_info()),
            Some(spl_interface_bump_a),
        )?;
    }
    cpi_a.invoke_signed(&[authority_seeds])?;

    let amount_b = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(pool_b_balance))
        .ok_or(SwapError::Overflow)?
        .checked_div(I64F64::from_num(lp_supply + MINIMUM_LIQUIDITY))
        .ok_or(SwapError::Underflow)?
        .floor()
        .to_num::<u64>();

    let decimals_b = ctx.accounts.mint_b.decimals;

    let mut cpi_b = TransferInterfaceCpi::new(
        amount_b,
        decimals_b,
        ctx.accounts.pool_account_b.to_account_info(),
        ctx.accounts.depositor_account_b.to_account_info(),
        ctx.accounts.pool_authority.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.mint_b.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    );
    if ctx.accounts.spl_interface_pda_b.is_some() {
        cpi_b = cpi_b.with_spl_interface(
            Some(ctx.accounts.mint_b.to_account_info()),
            Some(ctx.accounts.token_program.to_account_info()),
            ctx.accounts.spl_interface_pda_b.as_ref().map(|a| a.to_account_info()),
            Some(spl_interface_bump_b),
        )?;
    }
    cpi_b.invoke_signed(&[authority_seeds])?;

    if is_light_lp {
        BurnCpi {
            source: ctx.accounts.depositor_account_liquidity.to_account_info(),
            mint: ctx.accounts.mint_liquidity.to_account_info(),
            amount,
            authority: ctx.accounts.depositor.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            fee_payer: ctx.accounts.payer.to_account_info(),
        }
        .invoke()?;
    } else {
        token_interface::burn(
            CpiContext::new(
                ctx.accounts.liquidity_token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.mint_liquidity.to_account_info(),
                    from: ctx.accounts.depositor_account_liquidity.to_account_info(),
                    authority: ctx.accounts.depositor.to_account_info(),
                },
            ),
            amount,
        )?;
    }

    ctx.accounts.pool.lp_supply = ctx
        .accounts
        .pool
        .lp_supply
        .checked_sub(amount)
        .ok_or(SwapError::Underflow)?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawLiquidity<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump,
    )]
    pub amm: Account<'info, Amm>,

    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            pool.mint_a.key().as_ref(),
            pool.mint_b.key().as_ref(),
        ],
        bump,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: PDA verified by seeds constraint
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    pub depositor: Signer<'info>,

    /// CHECK: Can be SPL, T22, or Light
    #[account(mut)]
    pub mint_liquidity: UncheckedAccount<'info>,

    #[account(mut, mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut, mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: PDA verified by seeds constraint
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_A_SEED, pool.key().as_ref()],
        bump,
    )]
    pub pool_account_a: UncheckedAccount<'info>,

    /// CHECK: PDA verified by seeds constraint
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_B_SEED, pool.key().as_ref()],
        bump,
    )]
    pub pool_account_b: UncheckedAccount<'info>,

    /// CHECK: Can be SPL, T22, or Light
    #[account(mut)]
    pub depositor_account_liquidity: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = mint_a,
        token::authority = depositor,
    )]
    pub depositor_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_b,
        token::authority = depositor,
    )]
    pub depositor_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// Token program for mint_a and mint_b (SPL, T22, or Light).
    pub token_program: Interface<'info, TokenInterface>,
    /// Token program for LP mint (may differ from token_program).
    pub liquidity_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Validated by address constraint
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA derived by light-token: ["pool", mint_a]
    #[account(mut)]
    pub spl_interface_pda_a: Option<AccountInfo<'info>>,

    /// CHECK: SPL interface PDA derived by light-token: ["pool", mint_b]
    #[account(mut)]
    pub spl_interface_pda_b: Option<AccountInfo<'info>>,
}
