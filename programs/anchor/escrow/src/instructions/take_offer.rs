use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::{CloseAccountCpi, TransferInterfaceCpi, LIGHT_TOKEN_RENT_SPONSOR};
use light_token::utils::get_token_account_balance;

use crate::constants::{AUTH_SEED, OFFER_SEED, VAULT_SEED};
use crate::state::Offer;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TakeOfferParams {
    pub spl_interface_bump_a: u8,
    pub spl_interface_bump_b: u8,
}

#[derive(Accounts)]
pub struct TakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    /// CHECK: Authority PDA (writable for vault close)
    #[account(
        seeds = [AUTH_SEED],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    pub token_mint_a: InterfaceAccount<'info, Mint>,

    pub token_mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = token_mint_a,
        token::authority = taker,
    )]
    pub taker_token_account_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_mint_b,
        token::authority = taker,
    )]
    pub taker_token_account_b: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_mint_b,
        token::authority = maker,
    )]
    pub maker_token_account_b: InterfaceAccount<'info, TokenAccount>,

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

    #[account(
        mut,
        seeds = [VAULT_SEED, offer.key().as_ref()],
        bump,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light Token Program
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Light Token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light Token rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint A (vault->taker transfer)
    #[account(mut)]
    pub spl_interface_pda_a: Option<AccountInfo<'info>>,

    /// CHECK: SPL interface PDA for mint B (taker->maker transfer)
    #[account(mut)]
    pub spl_interface_pda_b: Option<AccountInfo<'info>>,
}

pub fn send_wanted_tokens_to_maker(ctx: &Context<TakeOffer>, params: &TakeOfferParams) -> Result<()> {
    let decimals_b = ctx.accounts.token_mint_b.decimals;
    let token_b_wanted_amount = ctx.accounts.offer.token_b_wanted_amount;

    let mut cpi = TransferInterfaceCpi::new(
        token_b_wanted_amount,
        decimals_b,
        ctx.accounts.taker_token_account_b.to_account_info(),
        ctx.accounts.maker_token_account_b.to_account_info(),
        ctx.accounts.taker.to_account_info(),
        ctx.accounts.taker.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.token_mint_b.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    );
    if ctx.accounts.spl_interface_pda_b.is_some() {
        cpi = cpi.with_spl_interface(
            Some(ctx.accounts.token_mint_b.to_account_info()),
            Some(ctx.accounts.token_program.to_account_info()),
            ctx.accounts.spl_interface_pda_b.as_ref().map(|a| a.to_account_info()),
            Some(params.spl_interface_bump_b),
        )?;
    }
    cpi.invoke()?;
    Ok(())
}

pub fn withdraw_from_vault(ctx: &Context<TakeOffer>, params: &TakeOfferParams) -> Result<()> {
    let offer = &ctx.accounts.offer;
    let authority_seeds: &[&[u8]] = &[AUTH_SEED, &[offer.auth_bump]];

    let vault_balance = get_token_account_balance(&ctx.accounts.vault.to_account_info())?;

    let decimals_a = ctx.accounts.token_mint_a.decimals;

    let mut cpi = TransferInterfaceCpi::new(
        vault_balance,
        decimals_a,
        ctx.accounts.vault.to_account_info(),
        ctx.accounts.taker_token_account_a.to_account_info(),
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.taker.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.token_mint_a.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    );
    if ctx.accounts.spl_interface_pda_a.is_some() {
        cpi = cpi.with_spl_interface(
            Some(ctx.accounts.token_mint_a.to_account_info()),
            Some(ctx.accounts.token_program.to_account_info()),
            ctx.accounts.spl_interface_pda_a.as_ref().map(|a| a.to_account_info()),
            Some(params.spl_interface_bump_a),
        )?;
    }
    cpi.invoke_signed(&[authority_seeds])?;

    msg!(
        "Offer taken: id={}, taker={}, transferred {} token_b to maker, received {} token_a",
        offer.id,
        ctx.accounts.taker.key(),
        offer.token_b_wanted_amount,
        vault_balance
    );

    Ok(())
}

pub fn close_vault(ctx: &Context<TakeOffer>) -> Result<()> {
    let auth_bump = ctx.accounts.offer.auth_bump;
    let authority_seeds: &[&[u8]] = &[AUTH_SEED, &[auth_bump]];

    CloseAccountCpi {
        token_program: ctx.accounts.light_token_program.to_account_info(),
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.taker.to_account_info(),
        owner: ctx.accounts.authority.to_account_info(),
        rent_sponsor: ctx.accounts.light_token_rent_sponsor.to_account_info(),
    }
    .invoke_signed(&[authority_seeds])?;
    Ok(())
}
