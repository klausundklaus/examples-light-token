use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_account::CreateAccountsProof;
use light_account::LightAccounts;
use light_token::instruction::{TransferInterfaceCpi, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

use crate::constants::{ANCHOR_DISCRIMINATOR, AUTH_SEED, OFFER_SEED, VAULT_SEED};
use crate::state::Offer;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MakeOfferParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub id: u64,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
    pub vault_bump: u8,
    pub spl_interface_bump_a: u8,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: MakeOfferParams)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority PDA for the vault
    #[account(
        seeds = [AUTH_SEED],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: Per-program rent sponsor
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(mint::token_program = token_program)]
    pub token_mint_a: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub token_mint_b: InterfaceAccount<'info, Mint>,

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

    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [VAULT_SEED, offer.key().as_ref()],
        bump,
    )]
    #[light_account(init,
        token::seeds = [VAULT_SEED, self.offer.key()],
        token::mint = token_mint_a,
        token::owner = authority,
        token::owner_seeds = [AUTH_SEED],
        token::bump = params.vault_bump
    )]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light Token Program
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Light Token config
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: Light Token rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token CPI authority
    #[account(mut)]
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint A
    #[account(mut)]
    pub spl_interface_pda_a: UncheckedAccount<'info>,
}

pub fn send_offered_tokens_to_vault<'info>(
    ctx: &Context<'_, '_, '_, 'info, MakeOffer<'info>>,
    params: &MakeOfferParams,
) -> Result<()> {
    let decimals = ctx.accounts.token_mint_a.decimals;

    let cpi = TransferInterfaceCpi::new(
        params.token_a_offered_amount,
        decimals,
        ctx.accounts.maker_token_account_a.to_account_info(),
        ctx.accounts.vault.to_account_info(),
        ctx.accounts.fee_payer.to_account_info(),
        ctx.accounts.fee_payer.to_account_info(),
        ctx.accounts.light_token_cpi_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    )
    .with_spl_interface(
        Some(ctx.accounts.token_mint_a.to_account_info()),
        Some(ctx.accounts.token_program.to_account_info()),
        Some(ctx.accounts.spl_interface_pda_a.to_account_info()),
        Some(params.spl_interface_bump_a),
    )
    ?;

    cpi.invoke()
}

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
