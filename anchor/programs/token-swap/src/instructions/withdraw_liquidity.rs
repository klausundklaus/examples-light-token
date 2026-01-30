use anchor_lang::prelude::*;
use fixed::types::I64F64;
use light_anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};
use light_token::instruction::LIGHT_TOKEN_RENT_SPONSOR;
use light_token::spl_interface::find_spl_interface_pda;
use light_token::utils::get_token_account_balance;

use crate::{
    constants::{
        AUTHORITY_SEED, LIQUIDITY_SEED, MINIMUM_LIQUIDITY, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED,
    },
    instructions::{transfer_tokens, SplInterfaceConfig},
    state::{Amm, Pool},
};

pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        &[authority_bump],
    ];

    // Get pool balances using Light Protocol's get_token_account_balance
    let pool_a_balance = get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
    let pool_b_balance = get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

    // Transfer tokens from the pool
    let amount_a = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(pool_a_balance))
        .unwrap()
        .checked_div(I64F64::from_num(
            ctx.accounts.mint_liquidity.supply + MINIMUM_LIQUIDITY,
        ))
        .unwrap()
        .floor()
        .to_num::<u64>();

    // Get SPL interface PDA bumps (derived from light-token program)
    let (_, spl_interface_bump_a) = find_spl_interface_pda(&ctx.accounts.mint_a.key(), false);
    let (_, spl_interface_bump_b) = find_spl_interface_pda(&ctx.accounts.mint_b.key(), false);

    let decimals_a = ctx.accounts.mint_a.decimals;
    let spl_interface_a = SplInterfaceConfig {
        mint: ctx.accounts.mint_a.to_account_info(),
        spl_token_program: ctx.accounts.token_program.to_account_info(),
        spl_interface_pda: ctx.accounts.spl_interface_pda_a.to_account_info(),
        spl_interface_pda_bump: spl_interface_bump_a,
    };
    transfer_tokens(
        amount_a,
        decimals_a,
        ctx.accounts.pool_account_a.to_account_info(),
        ctx.accounts.depositor_account_a.to_account_info(),
        ctx.accounts.mint_a.to_account_info(),
        ctx.accounts.pool_authority.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        Some(authority_seeds),
        Some(spl_interface_a),
    )?;

    let amount_b = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(pool_b_balance))
        .unwrap()
        .checked_div(I64F64::from_num(
            ctx.accounts.mint_liquidity.supply + MINIMUM_LIQUIDITY,
        ))
        .unwrap()
        .floor()
        .to_num::<u64>();

    let decimals_b = ctx.accounts.mint_b.decimals;
    let spl_interface_b = SplInterfaceConfig {
        mint: ctx.accounts.mint_b.to_account_info(),
        spl_token_program: ctx.accounts.token_program.to_account_info(),
        spl_interface_pda: ctx.accounts.spl_interface_pda_b.to_account_info(),
        spl_interface_pda_bump: spl_interface_bump_b,
    };
    transfer_tokens(
        amount_b,
        decimals_b,
        ctx.accounts.pool_account_b.to_account_info(),
        ctx.accounts.depositor_account_b.to_account_info(),
        ctx.accounts.mint_b.to_account_info(),
        ctx.accounts.pool_authority.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        Some(authority_seeds),
        Some(spl_interface_b),
    )?;

    // Burn the liquidity tokens (SPL or T22 token burn - never Light)
    // It will fail if the amount is invalid
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

    /// CHECK: Pool authority PDA - signer for SPL liquidity and Light token operations
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    /// The account paying for all rents
    pub depositor: Signer<'info>,

    /// Liquidity mint - always SPL or T22 (not Light)
    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
        mint::token_program = liquidity_token_program,
    )]
    pub mint_liquidity: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut, mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut, mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Pool token account A (Light Protocol token account)
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_A_SEED, pool.key().as_ref()],
        bump,
    )]
    pub pool_account_a: UncheckedAccount<'info>,

    /// CHECK: Pool token account B (Light Protocol token account)
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_B_SEED, pool.key().as_ref()],
        bump,
    )]
    pub pool_account_b: UncheckedAccount<'info>,

    /// Depositor's liquidity token account (can be SPL, T22, or Light)
    #[account(
        mut,
        token::mint = mint_liquidity,
        token::authority = depositor,
    )]
    pub depositor_account_liquidity: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Depositor's token account for mint A (can be SPL, T22, or Light)
    #[account(
        mut,
        token::mint = mint_a,
        token::authority = depositor,
    )]
    pub depositor_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Depositor's token account for mint B (can be SPL, T22, or Light)
    #[account(
        mut,
        token::mint = mint_b,
        token::authority = depositor,
    )]
    pub depositor_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Token program for mint_a and mint_b (SPL, T22, or Light)
    pub token_program: Interface<'info, TokenInterface>,
    /// Token program for liquidity mint (must be SPL or T22, not Light)
    pub liquidity_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light token program for CPI calls
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Light token rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: light-token CPI authority - must be writable for Light token CPI
    #[account(mut)]
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint A (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint_a]
    #[account(mut)]
    pub spl_interface_pda_a: UncheckedAccount<'info>,

    /// CHECK: SPL interface PDA for mint B (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint_b]
    #[account(mut)]
    pub spl_interface_pda_b: UncheckedAccount<'info>,
}
