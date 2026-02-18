use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::{TransferCheckedCpi, TransferInterfaceCpi, LIGHT_TOKEN_RENT_SPONSOR};

use crate::constants::{ANCHOR_DISCRIMINATOR, VAULT_SEED};
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

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint]
    #[account(mut)]
    pub spl_interface_pda: UncheckedAccount<'info>,
}

impl<'info> Contribute<'info> {
    pub fn contribute(&mut self, amount: u64, spl_interface_bump: u8) -> Result<()> {
        require!(
            amount >= 10_u64.checked_pow(self.mint_to_raise.decimals as u32).ok_or(FundraiserError::CalculationOverflow)?,
            FundraiserError::ContributionTooSmall
        );

        let max_contribution = self.fundraiser.amount_to_raise
            .checked_mul(MAX_CONTRIBUTION_PERCENTAGE)
            .and_then(|v| v.checked_div(PERCENTAGE_SCALER))
            .ok_or(FundraiserError::CalculationOverflow)?;
        require!(amount <= max_contribution, FundraiserError::ContributionTooBig);

        let current_time = Clock::get()?.unix_timestamp;
        let elapsed_days = ((current_time - self.fundraiser.time_started) / SECONDS_TO_DAYS) as u16;
        require!(
            elapsed_days < self.fundraiser.duration,
            FundraiserError::FundraiserEnded
        );

        require!(
            self.contributor_account.amount <= max_contribution
                && self.contributor_account.amount.checked_add(amount)
                    .map_or(false, |total| total <= max_contribution),
            FundraiserError::MaximumContributionsReached
        );

        let decimals = self.mint_to_raise.decimals;

        if self.spl_interface_pda.key() != Pubkey::default() {
            let cpi = TransferInterfaceCpi::new(
                amount,
                decimals,
                self.contributor_ata.to_account_info(),
                self.vault.to_account_info(),
                self.contributor.to_account_info(),
                self.contributor.to_account_info(),
                self.light_token_cpi_authority.to_account_info(),
                self.system_program.to_account_info(),
            )
            .with_spl_interface(
                Some(self.mint_to_raise.to_account_info()),
                Some(self.token_program.to_account_info()),
                Some(self.spl_interface_pda.to_account_info()),
                Some(spl_interface_bump),
            )
            ?;

            cpi.invoke()?;
        } else {
            TransferCheckedCpi {
                source: self.contributor_ata.to_account_info(),
                mint: self.mint_to_raise.to_account_info(),
                destination: self.vault.to_account_info(),
                amount,
                decimals,
                authority: self.contributor.to_account_info(),
                system_program: self.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: Some(self.contributor.to_account_info()),
            }
            .invoke()?;
        }

        self.fundraiser.current_amount = self.fundraiser.current_amount
            .checked_add(amount)
            .ok_or(FundraiserError::CalculationOverflow)?;
        self.contributor_account.amount = self.contributor_account.amount
            .checked_add(amount)
            .ok_or(FundraiserError::CalculationOverflow)?;

        Ok(())
    }
}
