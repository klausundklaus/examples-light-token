use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::LIGHT_TOKEN_RENT_SPONSOR;
use light_token::spl_interface::find_spl_interface_pda;

use crate::constants::{ANCHOR_DISCRIMINATOR, VAULT_SEED};
use crate::instructions::{transfer_tokens, SplInterfaceConfig};
use crate::state::{Contributor, Fundraiser};
use crate::{FundraiserError, MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER, SECONDS_TO_DAYS};

#[derive(Accounts)]
pub struct Contribute<'info> {
    #[account(mut)]
    pub contributor: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_to_raise: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = mint_to_raise,
        seeds = [b"fundraiser".as_ref(), fundraiser.maker.as_ref()],
        bump = fundraiser.bump,
    )]
    pub fundraiser: Account<'info, Fundraiser>,

    #[account(
        init_if_needed,
        payer = contributor,
        seeds = [b"contributor", fundraiser.key().as_ref(), contributor.key().as_ref()],
        bump,
        space = ANCHOR_DISCRIMINATOR + Contributor::INIT_SPACE,
    )]
    pub contributor_account: Account<'info, Contributor>,

    #[account(
        mut,
        token::mint = mint_to_raise,
        token::authority = contributor
    )]
    pub contributor_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Vault token account
    #[account(
        mut,
        seeds = [VAULT_SEED, fundraiser.key().as_ref()],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light token program for CPI calls
    pub light_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Light token rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: light-token CPI authority - must be writable for Light token CPI
    #[account(mut)]
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint]
    #[account(mut)]
    pub spl_interface_pda: UncheckedAccount<'info>,
}

impl<'info> Contribute<'info> {
    pub fn contribute(&mut self, amount: u64) -> Result<()> {
        // Check if the amount to contribute meets the minimum amount required
        require!(
            amount >= 1_u64.pow(self.mint_to_raise.decimals as u32),
            FundraiserError::ContributionTooSmall
        );

        // Check if the amount to contribute is less than the maximum allowed contribution
        require!(
            amount
                <= (self.fundraiser.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE)
                    / PERCENTAGE_SCALER,
            FundraiserError::ContributionTooBig
        );

        // Check if the fundraising duration has been reached
        let current_time = Clock::get()?.unix_timestamp;
        let elapsed_days = ((current_time - self.fundraiser.time_started) / SECONDS_TO_DAYS) as u16;
        require!(
            elapsed_days < self.fundraiser.duration,
            FundraiserError::FundraiserEnded
        );

        // Check if the maximum contributions per contributor have been reached
        require!(
            (self.contributor_account.amount
                <= (self.fundraiser.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE)
                    / PERCENTAGE_SCALER)
                && (self.contributor_account.amount + amount
                    <= (self.fundraiser.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE)
                        / PERCENTAGE_SCALER),
            FundraiserError::MaximumContributionsReached
        );

        // Get SPL interface PDA bump
        let (_, spl_interface_bump) = find_spl_interface_pda(&self.mint_to_raise.key(), false);

        // Transfer tokens from contributor to vault using Light Protocol
        let decimals = self.mint_to_raise.decimals;

        let spl_interface = SplInterfaceConfig {
            mint: self.mint_to_raise.to_account_info(),
            spl_token_program: self.token_program.to_account_info(),
            spl_interface_pda: self.spl_interface_pda.to_account_info(),
            spl_interface_pda_bump: spl_interface_bump,
        };

        transfer_tokens(
            amount,
            decimals,
            self.contributor_ata.to_account_info(),
            self.vault.to_account_info(),
            self.mint_to_raise.to_account_info(),
            self.contributor.to_account_info(),
            self.contributor.to_account_info(),
            self.light_token_cpi_authority.to_account_info(),
            self.system_program.to_account_info(),
            None,
            Some(spl_interface),
        )?;

        // Update the fundraiser and contributor accounts with the new amounts
        self.fundraiser.current_amount += amount;
        self.contributor_account.amount += amount;

        Ok(())
    }
}
