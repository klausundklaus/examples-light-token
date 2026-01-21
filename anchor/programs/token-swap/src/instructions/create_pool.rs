use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use light_anchor_spl::token_interface::TokenInterface;
use light_sdk::interface::CreateAccountsProof;
use light_token::anchor::LightAccounts;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED},
    state::{Amm, Pool},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePoolParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub pool_account_a_bump: u8,
    pub pool_account_b_bump: u8,
}

pub fn create_pool(ctx: Context<CreatePool>, _params: CreatePoolParams) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    pool.amm = ctx.accounts.amm.key();
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();

    Ok(())
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreatePoolParams)]
pub struct CreatePool<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump,
    )]
    pub amm: Box<Account<'info, Amm>>,

    #[account(
        init,
        payer = fee_payer,
        space = Pool::LEN,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
        ],
        bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: Read only authority
    #[account(
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority,
    )]
    pub mint_liquidity: Box<Account<'info, Mint>>,

    pub mint_a: Box<Account<'info, Mint>>,

    pub mint_b: Box<Account<'info, Mint>>,

    /// Pool token account A - created by light_account macro
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_A_SEED, pool.key().as_ref()],
        bump,
    )]
    #[light_account(init, token,
        authority = [POOL_ACCOUNT_A_SEED, self.pool.key(), &[params.pool_account_a_bump]],
        mint = mint_a,
        owner = pool_authority
    )]
    pub pool_account_a: UncheckedAccount<'info>,

    /// Pool token account B - created by light_account macro
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_B_SEED, pool.key().as_ref()],
        bump,
    )]
    #[light_account(init, token,
        authority = [POOL_ACCOUNT_B_SEED, self.pool.key(), &[params.pool_account_b_bump]],
        mint = mint_b,
        owner = pool_authority
    )]
    pub pool_account_b: UncheckedAccount<'info>,

    /// The account paying for all rents (also fee payer for Light Protocol)
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub light_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light token compressible config account
    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    /// Light token rent sponsor account
    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: light-token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,
}
