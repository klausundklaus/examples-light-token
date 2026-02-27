use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::TokenInterface;
use light_account::CreateAccountsProof;
use light_account::LightAccounts;
use light_token::instruction::{CreateTokenAccountCpi, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

use crate::{
    constants::{AUTHORITY_SEED, LP_MINT_SIGNER_SEED, POOL_ACCOUNT_A_SEED, POOL_ACCOUNT_B_SEED},
    state::{Amm, Pool},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePoolLightLpParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub pool_account_a_bump: u8,
    pub pool_account_b_bump: u8,
    pub lp_mint_signer_bump: u8,
    pub pool_authority_bump: u8,
}

pub fn create_pool_light_lp(
    ctx: Context<CreatePoolLightLp>,
    params: CreatePoolLightLpParams,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    pool.amm = ctx.accounts.amm.key();
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();
    pool.lp_supply = 0;

    let pool_key = ctx.accounts.pool.key();

    CreateTokenAccountCpi {
        payer: ctx.accounts.fee_payer.to_account_info(),
        account: ctx.accounts.pool_vault_a.to_account_info(),
        mint: ctx.accounts.mint_a.to_account_info(),
        owner: ctx.accounts.pool_authority.key(),
    }
    .rent_free(
        ctx.accounts.light_token_config.to_account_info(),
        ctx.accounts.light_token_rent_sponsor.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        &crate::ID,
    )
    .invoke_signed(&[
        POOL_ACCOUNT_A_SEED,
        pool_key.as_ref(),
        &[params.pool_account_a_bump],
    ])?;

    CreateTokenAccountCpi {
        payer: ctx.accounts.fee_payer.to_account_info(),
        account: ctx.accounts.pool_vault_b.to_account_info(),
        mint: ctx.accounts.mint_b.to_account_info(),
        owner: ctx.accounts.pool_authority.key(),
    }
    .rent_free(
        ctx.accounts.light_token_config.to_account_info(),
        ctx.accounts.light_token_rent_sponsor.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        &crate::ID,
    )
    .invoke_signed(&[
        POOL_ACCOUNT_B_SEED,
        pool_key.as_ref(),
        &[params.pool_account_b_bump],
    ])?;

    Ok(())
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreatePoolLightLpParams)]
pub struct CreatePoolLightLp<'info> {
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

    /// CHECK: PDA verified by seeds constraint
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    /// CHECK: PDA verified by seeds constraint
    #[account(
        seeds = [LP_MINT_SIGNER_SEED, pool.key().as_ref()],
        bump,
    )]
    pub lp_mint_signer: UncheckedAccount<'info>,

    /// CHECK: Initialized by light_account macro
    #[account(mut)]
    #[light_account(init,
        mint::signer = lp_mint_signer,
        mint::authority = pool_authority,
        mint::decimals = 6,
        mint::seeds = &[LP_MINT_SIGNER_SEED, self.pool.to_account_info().key.as_ref()],
        mint::bump = params.lp_mint_signer_bump,
        mint::name = b"LP Token".to_vec(),
        mint::symbol = b"LP".to_vec(),
        mint::uri = b"".to_vec(),
        mint::update_authority = pool_authority,
        mint::authority_seeds = &[AUTHORITY_SEED],
        mint::authority_bump = params.pool_authority_bump
    )]
    pub mint_liquidity: UncheckedAccount<'info>,

    /// CHECK: Validated by pool_vault_a light_account constraint
    pub mint_a: AccountInfo<'info>,

    /// CHECK: Validated by pool_vault_b light_account constraint
    pub mint_b: AccountInfo<'info>,

    /// CHECK: Created via CreateTokenAccountCpi in handler
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_A_SEED, pool.key().as_ref()],
        bump,
    )]
    #[light_account(token::seeds = [POOL_ACCOUNT_A_SEED, self.pool.key()], token::owner_seeds = [AUTHORITY_SEED])]
    pub pool_vault_a: UncheckedAccount<'info>,

    /// CHECK: Created via CreateTokenAccountCpi in handler
    #[account(
        mut,
        seeds = [POOL_ACCOUNT_B_SEED, pool.key().as_ref()],
        bump,
    )]
    #[light_account(token::seeds = [POOL_ACCOUNT_B_SEED, self.pool.key()], token::owner_seeds = [AUTHORITY_SEED])]
    pub pool_vault_b: UncheckedAccount<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Token program for mint_a and mint_b (SPL, Token 2022, or Light Token).
    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: Program-specific config for light mint creation
    pub compression_config: AccountInfo<'info>,

    /// CHECK: Validated by address constraint
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: Validated by address constraint
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
