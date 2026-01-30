use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenInterface};
use light_sdk::interface::CreateAccountsProof;
use light_token::anchor::LightAccounts;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

use crate::constants::{ANCHOR_DISCRIMINATOR, AUTH_SEED, VAULT_SEED};
use crate::state::Fundraiser;
use crate::{FundraiserError, MIN_AMOUNT_TO_RAISE};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub amount: u64,
    pub duration: u16,
    pub vault_bump: u8,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: InitializeParams)]
pub struct Initialize<'info> {
    /// The maker who creates the fundraiser (also the fee payer for Light Protocol)
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority PDA for the vault
    #[account(
        seeds = [AUTH_SEED],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_to_raise: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = fee_payer,
        space = ANCHOR_DISCRIMINATOR + Fundraiser::INIT_SPACE,
        seeds = [b"fundraiser", fee_payer.key().as_ref()],
        bump,
    )]
    pub fundraiser: Account<'info, Fundraiser>,

    /// The vault that holds the fundraiser tokens - created by the light_account macro
    #[account(
        mut,
        seeds = [VAULT_SEED, fundraiser.key().as_ref()],
        bump,
    )]
    #[light_account(init,
        token::seeds = [VAULT_SEED, self.fundraiser.key()],
        token::mint = mint_to_raise,
        token::owner = authority,
        token::bump = params.vault_bump,
        token::owner_seeds = [AUTH_SEED]
    )]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    /// Light token config account
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// Light token rent sponsor account
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// Light token program for CPI
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: light-token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, params: &InitializeParams, bumps: &InitializeBumps) -> Result<()> {
        // Check if the amount to raise meets the minimum amount required
        require!(
            params.amount >= MIN_AMOUNT_TO_RAISE.pow(self.mint_to_raise.decimals as u32),
            FundraiserError::InvalidAmount
        );

        // Initialize the fundraiser account
        self.fundraiser.set_inner(Fundraiser {
            maker: self.fee_payer.key(),
            mint_to_raise: self.mint_to_raise.key(),
            amount_to_raise: params.amount,
            current_amount: 0,
            time_started: Clock::get()?.unix_timestamp,
            duration: params.duration,
            bump: bumps.fundraiser,
        });

        Ok(())
    }
}
