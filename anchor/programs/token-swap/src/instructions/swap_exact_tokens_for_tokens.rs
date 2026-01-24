use anchor_lang::prelude::*;
use fixed::types::I64F64;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::RENT_SPONSOR;
use light_token::spl_interface::find_spl_interface_pda;
use light_token::utils::get_token_account_balance;

use crate::{
    constants::{AUTHORITY_SEED, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED},
    errors::*,
    instructions::{transfer_tokens, SplInterfaceConfig},
    state::{Amm, Pool},
};

pub fn swap_exact_tokens_for_tokens(
    ctx: Context<SwapExactTokensForTokens>,
    swap_a: bool,
    input_amount: u64,
    min_output_amount: u64,
) -> Result<()> {
    // Prevent depositing assets the depositor does not own
    let input = if swap_a && input_amount > ctx.accounts.trader_account_a.amount {
        ctx.accounts.trader_account_a.amount
    } else if !swap_a && input_amount > ctx.accounts.trader_account_b.amount {
        ctx.accounts.trader_account_b.amount
    } else {
        input_amount
    };

    // Get pool balances using Light Protocol's get_token_account_balance
    let pool_a_balance = get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
    let pool_b_balance = get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

    // Apply trading fee, used to compute the output
    let amm = &ctx.accounts.amm;
    let taxed_input = input - input * amm.fee as u64 / 10000;

    let output = if swap_a {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_b_balance))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_a_balance)
                    .checked_add(I64F64::from_num(taxed_input))
                    .unwrap(),
            )
            .unwrap()
    } else {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_a_balance))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_b_balance)
                    .checked_add(I64F64::from_num(taxed_input))
                    .unwrap(),
            )
            .unwrap()
    }
    .to_num::<u64>();

    if output < min_output_amount {
        return err!(TutorialError::OutputTooSmall);
    }

    // Compute the invariant before the trade
    let invariant = pool_a_balance * pool_b_balance;

    // Transfer tokens to the pool
    let authority_bump = ctx.bumps.pool_authority;
    let mint_a_key = ctx.accounts.mint_a.key();
    let mint_b_key = ctx.accounts.mint_b.key();
    let authority_seeds: &[&[u8]] = &[
        ctx.accounts.pool.amm.as_ref(),
        mint_a_key.as_ref(),
        mint_b_key.as_ref(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];

    let decimals_a = ctx.accounts.mint_a.decimals;
    let decimals_b = ctx.accounts.mint_b.decimals;

    // Get SPL interface PDA bumps (derived from light-token program)
    let (_, spl_interface_bump_a) = find_spl_interface_pda(&ctx.accounts.mint_a.key(), false);
    let (_, spl_interface_bump_b) = find_spl_interface_pda(&ctx.accounts.mint_b.key(), false);

    if swap_a {
        // Transfer token A from trader (SPL) to pool (Light) - needs SPL interface
        let spl_interface_a = SplInterfaceConfig {
            mint: ctx.accounts.mint_a.to_account_info(),
            spl_token_program: ctx.accounts.token_program.to_account_info(),
            spl_interface_pda: ctx.accounts.spl_interface_pda_a.to_account_info(),
            spl_interface_pda_bump: spl_interface_bump_a,
        };
        transfer_tokens(
            input,
            decimals_a,
            ctx.accounts.trader_account_a.to_account_info(),
            ctx.accounts.pool_account_a.to_account_info(),
            ctx.accounts.mint_a.to_account_info(),
            ctx.accounts.trader.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.light_token_cpi_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            None,
            Some(spl_interface_a),
        )?;
        // Transfer token B from pool (Light) to trader (SPL) - needs SPL interface
        let spl_interface_b = SplInterfaceConfig {
            mint: ctx.accounts.mint_b.to_account_info(),
            spl_token_program: ctx.accounts.token_program.to_account_info(),
            spl_interface_pda: ctx.accounts.spl_interface_pda_b.to_account_info(),
            spl_interface_pda_bump: spl_interface_bump_b,
        };
        transfer_tokens(
            output,
            decimals_b,
            ctx.accounts.pool_account_b.to_account_info(),
            ctx.accounts.trader_account_b.to_account_info(),
            ctx.accounts.mint_b.to_account_info(),
            ctx.accounts.pool_authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.light_token_cpi_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            Some(authority_seeds),
            Some(spl_interface_b),
        )?;
    } else {
        // Transfer token B from trader (SPL) to pool (Light) - needs SPL interface
        let spl_interface_b = SplInterfaceConfig {
            mint: ctx.accounts.mint_b.to_account_info(),
            spl_token_program: ctx.accounts.token_program.to_account_info(),
            spl_interface_pda: ctx.accounts.spl_interface_pda_b.to_account_info(),
            spl_interface_pda_bump: spl_interface_bump_b,
        };
        transfer_tokens(
            input,
            decimals_b,
            ctx.accounts.trader_account_b.to_account_info(),
            ctx.accounts.pool_account_b.to_account_info(),
            ctx.accounts.mint_b.to_account_info(),
            ctx.accounts.trader.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.light_token_cpi_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            None,
            Some(spl_interface_b),
        )?;
        // Transfer token A from pool (Light) to trader (SPL) - needs SPL interface
        let spl_interface_a = SplInterfaceConfig {
            mint: ctx.accounts.mint_a.to_account_info(),
            spl_token_program: ctx.accounts.token_program.to_account_info(),
            spl_interface_pda: ctx.accounts.spl_interface_pda_a.to_account_info(),
            spl_interface_pda_bump: spl_interface_bump_a,
        };
        transfer_tokens(
            output,
            decimals_a,
            ctx.accounts.pool_account_a.to_account_info(),
            ctx.accounts.trader_account_a.to_account_info(),
            ctx.accounts.mint_a.to_account_info(),
            ctx.accounts.pool_authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.light_token_cpi_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            Some(authority_seeds),
            Some(spl_interface_a),
        )?;
    }

    msg!(
        "Traded {} tokens ({} after fees) for {}",
        input,
        taxed_input,
        output
    );

    // Verify the invariant still holds
    // Reload accounts because of the CPIs
    // We tolerate if the new invariant is higher because it means a rounding error for LPs
    let new_pool_a_balance =
        get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
            .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
    let new_pool_b_balance =
        get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
            .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

    if invariant > new_pool_a_balance * new_pool_b_balance {
        return err!(TutorialError::InvariantViolated);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct SwapExactTokensForTokens<'info> {
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
        has_one = amm,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: Pool authority PDA - signer for pool token transfers (readonly)
    #[account(
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    /// The account doing the swap
    /// Must be writable for compressible token rent top-ups
    #[account(mut)]
    pub trader: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
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

    /// Trader's token account for mint A (can be SPL, T22, or Light)
    #[account(
        mut,
        token::mint = mint_a,
        token::authority = trader,
    )]
    pub trader_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Trader's token account for mint B (can be SPL, T22, or Light)
    #[account(
        mut,
        token::mint = mint_b,
        token::authority = trader,
    )]
    pub trader_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Token program (SPL, T22, or Light)
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light token program for CPI calls
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Light token rent sponsor
    #[account(mut, address = RENT_SPONSOR)]
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
