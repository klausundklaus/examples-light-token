use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount},
};
use light_anchor_spl::token_interface::TokenInterface;
use fixed::types::I64F64;
use light_token::instruction::RENT_SPONSOR;
use light_token::spl_interface::find_spl_interface_pda;
use light_token::utils::get_token_account_balance;

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED, MINIMUM_LIQUIDITY, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED},
    errors::TutorialError,
    instructions::{transfer_tokens, SplInterfaceConfig},
    state::Pool,
};

pub fn deposit_liquidity(
    ctx: Context<DepositLiquidity>,
    amount_a: u64,
    amount_b: u64,
) -> Result<()> {
    // Get pool balances using Light Protocol's get_token_account_balance
    let pool_a_balance = get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
    let pool_b_balance = get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

    // Prevent depositing assets the depositor does not own
    let mut amount_a = if amount_a > ctx.accounts.depositor_account_a.amount {
        ctx.accounts.depositor_account_a.amount
    } else {
        amount_a
    };
    let mut amount_b = if amount_b > ctx.accounts.depositor_account_b.amount {
        ctx.accounts.depositor_account_b.amount
    } else {
        amount_b
    };

    // Making sure they are provided in the same proportion as existing liquidity
    // Defining pool creation like this allows attackers to frontrun pool creation with bad ratios
    let pool_creation = pool_a_balance == 0 && pool_b_balance == 0;
    (amount_a, amount_b) = if pool_creation {
        // Add as is if there is no liquidity
        (amount_a, amount_b)
    } else {
        let ratio = I64F64::from_num(pool_a_balance)
            .checked_mul(I64F64::from_num(pool_b_balance))
            .unwrap();
        if pool_a_balance > pool_b_balance {
            (
                I64F64::from_num(amount_b)
                    .checked_mul(ratio)
                    .unwrap()
                    .to_num::<u64>(),
                amount_b,
            )
        } else {
            (
                amount_a,
                I64F64::from_num(amount_a)
                    .checked_div(ratio)
                    .unwrap()
                    .to_num::<u64>(),
            )
        }
    };

    // Computing the amount of liquidity about to be deposited
    let mut liquidity = I64F64::from_num(amount_a)
        .checked_mul(I64F64::from_num(amount_b))
        .unwrap()
        .sqrt()
        .to_num::<u64>();

    // Lock some minimum liquidity on the first deposit
    if pool_creation {
        if liquidity < MINIMUM_LIQUIDITY {
            return err!(TutorialError::DepositTooSmall);
        }

        liquidity -= MINIMUM_LIQUIDITY;
    }

    // Transfer tokens to the pool using Light Protocol
    let decimals_a = ctx.accounts.mint_a.decimals;
    let decimals_b = ctx.accounts.mint_b.decimals;

    // Get SPL interface PDA bumps (derived from light-token program, not our program)
    let (_, spl_interface_bump_a) =
        find_spl_interface_pda(&ctx.accounts.mint_a.key(), false);
    let (_, spl_interface_bump_b) =
        find_spl_interface_pda(&ctx.accounts.mint_b.key(), false);

    // SPL interface config for SPL->Light transfers
    let spl_interface_a = SplInterfaceConfig {
        mint: ctx.accounts.mint_a.to_account_info(),
        spl_token_program: ctx.accounts.token_program.to_account_info(),
        spl_interface_pda: ctx.accounts.spl_interface_pda_a.to_account_info(),
        spl_interface_pda_bump: spl_interface_bump_a,
    };

    transfer_tokens(
        amount_a,
        decimals_a,
        ctx.accounts.depositor_account_a.to_account_info(),
        ctx.accounts.pool_account_a.to_account_info(),
        ctx.accounts.depositor.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        None,
        Some(spl_interface_a),
    )?;

    let spl_interface_b = SplInterfaceConfig {
        mint: ctx.accounts.mint_b.to_account_info(),
        spl_token_program: ctx.accounts.token_program.to_account_info(),
        spl_interface_pda: ctx.accounts.spl_interface_pda_b.to_account_info(),
        spl_interface_pda_bump: spl_interface_bump_b,
    };

    transfer_tokens(
        amount_b,
        decimals_b,
        ctx.accounts.depositor_account_b.to_account_info(),
        ctx.accounts.pool_account_b.to_account_info(),
        ctx.accounts.depositor.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        None,
        Some(spl_interface_b),
    )?;

    // Mint the liquidity to user (standard SPL token mint)
    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_liquidity.to_account_info(),
                to: ctx.accounts.depositor_account_liquidity.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        liquidity,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct DepositLiquidity<'info> {
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
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: Read only authority
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

    /// The account paying for all rents
    pub depositor: Signer<'info>,

    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
    )]
    pub mint_liquidity: Box<Account<'info, Mint>>,

    pub mint_a: Box<Account<'info, Mint>>,

    pub mint_b: Box<Account<'info, Mint>>,

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

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_liquidity,
        associated_token::authority = depositor,
    )]
    pub depositor_account_liquidity: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = depositor,
    )]
    pub depositor_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = depositor,
    )]
    pub depositor_account_b: Box<Account<'info, TokenAccount>>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// Light token program for CPI calls
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Light token rent sponsor
    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: light-token CPI authority
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
