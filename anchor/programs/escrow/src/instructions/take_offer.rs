use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::{CloseAccountCpi, RENT_SPONSOR};
use light_token::utils::get_token_account_balance;

use crate::constants::{AUTH_SEED, OFFER_SEED, VAULT_SEED};
use crate::instructions::transfer_tokens;
use crate::state::Offer;

#[derive(Accounts)]
pub struct TakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// The maker who created the offer
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    /// CHECK: Authority PDA for the vault (mutable for close_vault write_top_up)
    #[account(
        mut,
        seeds = [AUTH_SEED.as_bytes()],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// The mint of the token that was offered (token A)
    pub token_mint_a: InterfaceAccount<'info, Mint>,

    /// The mint of the token wanted (token B)
    pub token_mint_b: InterfaceAccount<'info, Mint>,

    /// The taker's token account for token A (receives offered tokens)
    #[account(
        mut,
        token::mint = token_mint_a,
        token::authority = taker,
    )]
    pub taker_token_account_a: InterfaceAccount<'info, TokenAccount>,

    /// The taker's token account for token B (sends wanted tokens)
    #[account(
        mut,
        token::mint = token_mint_b,
        token::authority = taker,
    )]
    pub taker_token_account_b: InterfaceAccount<'info, TokenAccount>,

    /// The maker's token account for token B (receives wanted tokens)
    #[account(
        mut,
        token::mint = token_mint_b,
        token::authority = maker,
    )]
    pub maker_token_account_b: InterfaceAccount<'info, TokenAccount>,

    /// The offer account
    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = token_mint_a,
        has_one = token_mint_b,
        seeds = [OFFER_SEED, maker.key().as_ref(), offer.id.to_le_bytes().as_ref()],
        bump,
    )]
    pub offer: Account<'info, Offer>,

    /// The vault holding the offered tokens
    #[account(
        mut,
        seeds = [VAULT_SEED, offer.key().as_ref()],
        bump,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// CHECK: light-token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light token rent sponsor for closing vault
    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,
}

/// Transfer token B from taker to maker
pub fn send_wanted_tokens_to_maker(ctx: &Context<TakeOffer>) -> Result<()> {
    let decimals_b = ctx.accounts.token_mint_b.decimals;
    let token_b_wanted_amount = ctx.accounts.offer.token_b_wanted_amount;

    transfer_tokens(
        token_b_wanted_amount,
        decimals_b,
        ctx.accounts.taker_token_account_b.to_account_info(),
        ctx.accounts.maker_token_account_b.to_account_info(),
        ctx.accounts.taker.to_account_info(),
        ctx.accounts.taker.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        None,
    )
}

/// Withdraw all tokens from vault to taker
pub fn withdraw_from_vault(ctx: &Context<TakeOffer>) -> Result<()> {
    let offer = &ctx.accounts.offer;
    let offer_id = offer.id;
    let token_b_wanted_amount = offer.token_b_wanted_amount;
    let auth_bump = offer.auth_bump;

    // Build signer seeds for authority
    let authority_seeds = &[AUTH_SEED.as_bytes(), &[auth_bump]];

    // Get the vault balance
    let vault_balance = get_token_account_balance(&ctx.accounts.vault.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

    let decimals_a = ctx.accounts.token_mint_a.decimals;

    transfer_tokens(
        vault_balance,
        decimals_a,
        ctx.accounts.vault.to_account_info(),
        ctx.accounts.taker_token_account_a.to_account_info(),
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.taker.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        Some(authority_seeds),
    )?;

    msg!(
        "Offer taken: id={}, taker={}, transferred {} token_b to maker, received {} token_a",
        offer_id,
        ctx.accounts.taker.key(),
        token_b_wanted_amount,
        vault_balance
    );

    Ok(())
}

/// Close the vault after withdrawal, returning rent to taker
pub fn close_vault(ctx: &Context<TakeOffer>) -> Result<()> {
    let auth_bump = ctx.accounts.offer.auth_bump;
    let authority_seeds: &[&[u8]] = &[AUTH_SEED.as_bytes(), &[auth_bump]];

    CloseAccountCpi {
        token_program: ctx.accounts.token_program.to_account_info(),
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.taker.to_account_info(),
        owner: ctx.accounts.authority.to_account_info(),
        rent_sponsor: ctx.accounts.light_token_rent_sponsor.to_account_info(),
    }
    .invoke_signed(&[authority_seeds])
    .map_err(|e| anchor_lang::prelude::ProgramError::from(e).into())
}
