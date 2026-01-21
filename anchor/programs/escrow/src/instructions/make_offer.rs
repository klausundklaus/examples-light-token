use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_sdk::interface::CreateAccountsProof;
use light_token::anchor::LightAccounts;
use light_token::instruction::{
    CreateTokenAccountCpi, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR,
};

use crate::constants::{ANCHOR_DISCRIMINATOR, AUTH_SEED, OFFER_SEED, VAULT_SEED};
use crate::state::Offer;
use crate::instructions::transfer_tokens;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MakeOfferParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub id: u64,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: MakeOfferParams)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority PDA for the vault
    #[account(
        seeds = [AUTH_SEED.as_bytes()],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// The mint of the token being offered
    #[account(mint::token_program = token_program)]
    pub token_mint_a: InterfaceAccount<'info, Mint>,

    /// The mint of the token wanted in exchange
    #[account(mint::token_program = token_program)]
    pub token_mint_b: InterfaceAccount<'info, Mint>,

    /// The maker's token account for token A
    #[account(
        mut,
        token::mint = token_mint_a,
        token::authority = fee_payer,
    )]
    pub maker_token_account_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = fee_payer,
        space = ANCHOR_DISCRIMINATOR + Offer::INIT_SPACE,
        seeds = [OFFER_SEED, fee_payer.key().as_ref(), params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub offer: Account<'info, Offer>,

    /// The vault that holds the offered tokens - created by the light_account macro
    #[account(
        mut,
        seeds = [VAULT_SEED, offer.key().as_ref()],
        bump,
    )]
    #[light_account(init, token,
        authority = [AUTH_SEED.as_bytes()],
        mint = token_mint_a,
        owner = authority
    )]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
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

/// Transfer tokens from maker to vault
pub fn send_offered_tokens_to_vault<'info>(
    ctx: &Context<'_, '_, '_, 'info, MakeOffer<'info>>,
    params: &MakeOfferParams,
) -> Result<()> {
    let decimals = ctx.accounts.token_mint_a.decimals;

    transfer_tokens(
        params.token_a_offered_amount,
        decimals,
        ctx.accounts.maker_token_account_a.to_account_info(),
        ctx.accounts.vault.to_account_info(),
        ctx.accounts.fee_payer.to_account_info(),
        ctx.accounts.fee_payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        None,
    )
}

/// Save the details of the offer to the offer account
pub fn save_offer<'info>(
    ctx: &mut Context<'_, '_, '_, 'info, MakeOffer<'info>>,
    params: &MakeOfferParams,
) -> Result<()> {
    let offer = &mut ctx.accounts.offer;
    offer.id = params.id;
    offer.maker = ctx.accounts.fee_payer.key();
    offer.token_mint_a = ctx.accounts.token_mint_a.key();
    offer.token_mint_b = ctx.accounts.token_mint_b.key();
    offer.token_b_wanted_amount = params.token_b_wanted_amount;
    offer.auth_bump = ctx.bumps.authority;

    msg!(
        "Offer created: id={}, maker={}, offered={} token_a, wants={} token_b",
        params.id,
        ctx.accounts.fee_payer.key(),
        params.token_a_offered_amount,
        params.token_b_wanted_amount
    );
    Ok(())
}

