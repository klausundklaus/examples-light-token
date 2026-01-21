use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenInterface};
use light_sdk::interface::CreateAccountsProof;
use light_token::anchor::LightAccounts;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

use crate::constants::{ANCHOR_DISCRIMINATOR, VAULT_SEED};
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
    #[light_account(init, token,
        authority = [VAULT_SEED, self.fundraiser.key(), &[params.vault_bump]],
        mint = mint_to_raise,
        owner = fundraiser
    )]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light token program for CPI calls
    pub light_token_program: Interface<'info, TokenInterface>,

    /// Light token compressible config account
    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    /// Light token rent sponsor account
    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

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
