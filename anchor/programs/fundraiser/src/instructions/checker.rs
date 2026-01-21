use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::RENT_SPONSOR;
use light_token::spl_interface::find_spl_interface_pda;
use light_token::utils::get_token_account_balance;

use crate::constants::VAULT_SEED;
use crate::instructions::{transfer_tokens, SplInterfaceConfig};
use crate::state::Fundraiser;
use crate::FundraiserError;

#[derive(Accounts)]
pub struct CheckContributions<'info> {
    /// The maker who checks and claims the fundraiser (also the fee payer for Light Protocol)
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_to_raise: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = mint_to_raise,
        seeds = [b"fundraiser".as_ref(), fee_payer.key().as_ref()],
        bump = fundraiser.bump,
        close = fee_payer,
    )]
    pub fundraiser: Account<'info, Fundraiser>,

    /// CHECK: Vault token account (Light token account)
    #[account(
        mut,
        seeds = [VAULT_SEED, fundraiser.key().as_ref()],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// Maker's SPL ATA - must be pre-created before calling this instruction
    /// Receives funds from the Light vault via SPL interface
    #[account(
        mut,
        token::mint = mint_to_raise,
        token::authority = fee_payer,
    )]
    pub maker_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// Light token program for CPI calls
    pub light_token_program: Interface<'info, TokenInterface>,

    /// Light token rent sponsor account
    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: light-token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint]
    #[account(mut)]
    pub spl_interface_pda: UncheckedAccount<'info>,
}

impl<'info> CheckContributions<'info> {
    pub fn check_contributions(&self, _bumps: &CheckContributionsBumps) -> Result<()> {
        // Get the vault balance
        let vault_balance = get_token_account_balance(&self.vault.to_account_info())
            .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

        // Check if the target amount has been met
        require!(
            vault_balance >= self.fundraiser.amount_to_raise,
            FundraiserError::TargetNotMet
        );

        // Build signer seeds for the fundraiser PDA (which is the vault's owner)
        let fundraiser_seeds: &[&[u8]] = &[
            b"fundraiser".as_ref(),
            self.fee_payer.to_account_info().key.as_ref(),
            &[self.fundraiser.bump],
        ];

        // Get SPL interface PDA bump
        let (_, spl_interface_bump) = find_spl_interface_pda(&self.mint_to_raise.key(), false);

        // Transfer all funds from vault to maker
        // vault (Light) -> maker_ata (SPL ATA) - needs SPL interface for Light->SPL transfer
        let decimals = self.mint_to_raise.decimals;

        let spl_interface = SplInterfaceConfig {
            mint: self.mint_to_raise.to_account_info(),
            spl_token_program: self.token_program.to_account_info(),
            spl_interface_pda: self.spl_interface_pda.to_account_info(),
            spl_interface_pda_bump: spl_interface_bump,
        };

        transfer_tokens(
            vault_balance,
            decimals,
            self.vault.to_account_info(),
            self.maker_ata.to_account_info(),
            self.fundraiser.to_account_info(),
            self.fee_payer.to_account_info(),
            self.light_token_cpi_authority.to_account_info(),
            self.system_program.to_account_info(),
            Some(fundraiser_seeds),
            Some(spl_interface),
        )?;

        Ok(())
    }
}
