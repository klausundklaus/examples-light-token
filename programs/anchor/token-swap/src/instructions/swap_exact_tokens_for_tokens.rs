use anchor_lang::prelude::*;
use fixed::types::I64F64;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::{TransferCheckedCpi, TransferInterfaceCpi, LIGHT_TOKEN_RENT_SPONSOR};
use light_token::utils::get_token_account_balance;

use crate::{
    constants::{AUTHORITY_SEED, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED},
    errors::*,
    state::{Amm, Pool},
};

pub fn swap_exact_tokens_for_tokens(
    ctx: Context<SwapExactTokensForTokens>,
    swap_a: bool,
    input_amount: u64,
    min_output_amount: u64,
    spl_interface_bump_a: u8,
    spl_interface_bump_b: u8,
) -> Result<()> {
    let input = if swap_a && input_amount > ctx.accounts.trader_account_a.amount {
        ctx.accounts.trader_account_a.amount
    } else if !swap_a && input_amount > ctx.accounts.trader_account_b.amount {
        ctx.accounts.trader_account_b.amount
    } else {
        input_amount
    };

    let pool_a_balance = get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
    let pool_b_balance = get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

    let amm = &ctx.accounts.amm;
    let fee_amount = (input as u128 * amm.fee as u128 / 10000) as u64;
    let taxed_input = input - fee_amount;

    let output = if swap_a {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_b_balance))
            .ok_or(SwapError::Overflow)?
            .checked_div(
                I64F64::from_num(pool_a_balance)
                    .checked_add(I64F64::from_num(taxed_input))
                    .ok_or(SwapError::Overflow)?,
            )
            .ok_or(SwapError::Underflow)?
    } else {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_a_balance))
            .ok_or(SwapError::Overflow)?
            .checked_div(
                I64F64::from_num(pool_b_balance)
                    .checked_add(I64F64::from_num(taxed_input))
                    .ok_or(SwapError::Overflow)?,
            )
            .ok_or(SwapError::Underflow)?
    }
    .to_num::<u64>();

    if output < min_output_amount {
        return err!(SwapError::OutputTooSmall);
    }

    let invariant = (pool_a_balance as u128) * (pool_b_balance as u128);

    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        &[authority_bump],
    ];

    let decimals_a = ctx.accounts.mint_a.decimals;
    let decimals_b = ctx.accounts.mint_b.decimals;

    let is_spl = ctx.accounts.spl_interface_pda_a.key() != Pubkey::default();

    if swap_a {
        if !is_spl {
            // fee_payer: Some ensures authority is readonly (required for PDA with account data)
            TransferCheckedCpi {
                source: ctx.accounts.trader_account_a.to_account_info(),
                mint: ctx.accounts.mint_a.to_account_info(),
                destination: ctx.accounts.pool_account_a.to_account_info(),
                amount: input,
                decimals: decimals_a,
                authority: ctx.accounts.trader.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: Some(ctx.accounts.payer.to_account_info()),
            }
            .invoke()
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            TransferCheckedCpi {
                source: ctx.accounts.pool_account_b.to_account_info(),
                mint: ctx.accounts.mint_b.to_account_info(),
                destination: ctx.accounts.trader_account_b.to_account_info(),
                amount: output,
                decimals: decimals_b,
                authority: ctx.accounts.pool_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: Some(ctx.accounts.payer.to_account_info()),
            }
            .invoke_signed(&[authority_seeds])
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        } else {
            let cpi_input = TransferInterfaceCpi::new(
                input,
                decimals_a,
                ctx.accounts.trader_account_a.to_account_info(),
                ctx.accounts.pool_account_a.to_account_info(),
                ctx.accounts.trader.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.light_token_cpi_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            )
            .with_spl_interface(
                Some(ctx.accounts.mint_a.to_account_info()),
                Some(ctx.accounts.token_program.to_account_info()),
                Some(ctx.accounts.spl_interface_pda_a.to_account_info()),
                Some(spl_interface_bump_a),
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            cpi_input.invoke()
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            let cpi_output = TransferInterfaceCpi::new(
                output,
                decimals_b,
                ctx.accounts.pool_account_b.to_account_info(),
                ctx.accounts.trader_account_b.to_account_info(),
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.light_token_cpi_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            )
            .with_spl_interface(
                Some(ctx.accounts.mint_b.to_account_info()),
                Some(ctx.accounts.token_program.to_account_info()),
                Some(ctx.accounts.spl_interface_pda_b.to_account_info()),
                Some(spl_interface_bump_b),
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            cpi_output.invoke_signed(&[authority_seeds])
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        }
    } else {
        if !is_spl {
            TransferCheckedCpi {
                source: ctx.accounts.trader_account_b.to_account_info(),
                mint: ctx.accounts.mint_b.to_account_info(),
                destination: ctx.accounts.pool_account_b.to_account_info(),
                amount: input,
                decimals: decimals_b,
                authority: ctx.accounts.trader.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: Some(ctx.accounts.payer.to_account_info()),
            }
            .invoke()
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            TransferCheckedCpi {
                source: ctx.accounts.pool_account_a.to_account_info(),
                mint: ctx.accounts.mint_a.to_account_info(),
                destination: ctx.accounts.trader_account_a.to_account_info(),
                amount: output,
                decimals: decimals_a,
                authority: ctx.accounts.pool_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: Some(ctx.accounts.payer.to_account_info()),
            }
            .invoke_signed(&[authority_seeds])
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        } else {
            let cpi_input = TransferInterfaceCpi::new(
                input,
                decimals_b,
                ctx.accounts.trader_account_b.to_account_info(),
                ctx.accounts.pool_account_b.to_account_info(),
                ctx.accounts.trader.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.light_token_cpi_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            )
            .with_spl_interface(
                Some(ctx.accounts.mint_b.to_account_info()),
                Some(ctx.accounts.token_program.to_account_info()),
                Some(ctx.accounts.spl_interface_pda_b.to_account_info()),
                Some(spl_interface_bump_b),
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            cpi_input.invoke()
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            let cpi_output = TransferInterfaceCpi::new(
                output,
                decimals_a,
                ctx.accounts.pool_account_a.to_account_info(),
                ctx.accounts.trader_account_a.to_account_info(),
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.light_token_cpi_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            )
            .with_spl_interface(
                Some(ctx.accounts.mint_a.to_account_info()),
                Some(ctx.accounts.token_program.to_account_info()),
                Some(ctx.accounts.spl_interface_pda_a.to_account_info()),
                Some(spl_interface_bump_a),
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            cpi_output.invoke_signed(&[authority_seeds])
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        }
    }

    msg!(
        "Traded {} tokens ({} after fees) for {}",
        input,
        taxed_input,
        output
    );

    // Higher invariant OK - rounding benefits LPs
    let new_pool_a_balance =
        get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
            .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
    let new_pool_b_balance =
        get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
            .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

    if invariant > (new_pool_a_balance as u128) * (new_pool_b_balance as u128) {
        return err!(SwapError::InvariantViolated);
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

    /// CHECK: PDA verified by seeds constraint
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(mut)]
    pub trader: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
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

    #[account(
        mut,
        token::mint = mint_a,
        token::authority = trader,
    )]
    pub trader_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_b,
        token::authority = trader,
    )]
    pub trader_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// Token program for mint_a and mint_b (SPL, T22, or Light).
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Validated by address constraint
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    #[account(mut)]
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA derived by light-token: ["pool", mint_a]
    #[account(mut)]
    pub spl_interface_pda_a: Option<AccountInfo<'info>>,

    /// CHECK: SPL interface PDA derived by light-token: ["pool", mint_b]
    #[account(mut)]
    pub spl_interface_pda_b: Option<AccountInfo<'info>>,
}
