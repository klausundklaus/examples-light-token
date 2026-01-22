use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::RENT_SPONSOR;
use light_token::spl_interface::find_spl_interface_pda;
use light_token::utils::get_token_account_balance;

use crate::constants::VAULT_SEED;
use crate::instructions::{transfer_tokens, SplInterfaceConfig};
use crate::state::{Contributor, Fundraiser};
use crate::{FundraiserError, SECONDS_TO_DAYS};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub contributor: Signer<'info>,

    pub maker: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_to_raise: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = mint_to_raise,
        seeds = [b"fundraiser", maker.key().as_ref()],
        bump = fundraiser.bump,
    )]
    pub fundraiser: Account<'info, Fundraiser>,

    #[account(
        mut,
        seeds = [b"contributor", fundraiser.key().as_ref(), contributor.key().as_ref()],
        bump,
        close = contributor,
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
    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: light-token CPI authority - must be writable for Light token CPI
    #[account(mut)]
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint]
    #[account(mut)]
    pub spl_interface_pda: UncheckedAccount<'info>,
}

impl<'info> Refund<'info> {
    pub fn refund(&mut self, _bumps: &RefundBumps) -> Result<()> {
        // Check if the fundraising duration has been reached
        let current_time = Clock::get()?.unix_timestamp;

        require!(
            self.fundraiser.duration
                >= ((current_time - self.fundraiser.time_started) / SECONDS_TO_DAYS) as u16,
            FundraiserError::FundraiserNotEnded
        );

        // Get the actual vault balance (matches original behavior)
        let vault_balance = get_token_account_balance(&self.vault.to_account_info())
            .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

        require!(
            vault_balance < self.fundraiser.amount_to_raise,
            FundraiserError::TargetMet
        );

        // Build signer seeds for the fundraiser PDA (which is the vault's owner)
        let fundraiser_seeds: &[&[u8]] = &[
            b"fundraiser".as_ref(),
            self.maker.to_account_info().key.as_ref(),
            &[self.fundraiser.bump],
        ];

        // Get SPL interface PDA bump
        let (_, spl_interface_bump) = find_spl_interface_pda(&self.mint_to_raise.key(), false);

        // Transfer the funds back to the contributor
        // vault (Light) -> contributor_ata (SPL) - needs SPL interface
        let decimals = self.mint_to_raise.decimals;
        let refund_amount = self.contributor_account.amount;

        let spl_interface = SplInterfaceConfig {
            mint: self.mint_to_raise.to_account_info(),
            spl_token_program: self.token_program.to_account_info(),
            spl_interface_pda: self.spl_interface_pda.to_account_info(),
            spl_interface_pda_bump: spl_interface_bump,
        };

        transfer_tokens(
            refund_amount,
            decimals,
            self.vault.to_account_info(),
            self.contributor_ata.to_account_info(),
            self.mint_to_raise.to_account_info(),
            self.fundraiser.to_account_info(),
            self.contributor.to_account_info(),
            self.light_token_cpi_authority.to_account_info(),
            self.system_program.to_account_info(),
            Some(fundraiser_seeds),
            Some(spl_interface),
        )?;

        // Update the fundraiser state by reducing the amount contributed
        self.fundraiser.current_amount -= refund_amount;

        Ok(())
    }
}
