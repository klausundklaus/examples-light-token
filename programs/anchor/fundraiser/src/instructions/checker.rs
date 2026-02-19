use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::{TransferInterfaceCpi, LIGHT_TOKEN_RENT_SPONSOR};
use light_token::utils::get_token_account_balance;

use crate::constants::{AUTH_SEED, VAULT_SEED};
use crate::state::Fundraiser;
use crate::FundraiserError;

#[derive(Accounts)]
pub struct CheckContributions<'info> {
    /// The maker who checks and claims the fundraiser (also the fee payer for Light Protocol)
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority PDA
    #[account(seeds = [AUTH_SEED], bump)]
    pub authority: UncheckedAccount<'info>,

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

    /// CHECK: Light token rent sponsor - validated by address constraint
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint]
    #[account(mut)]
    pub spl_interface_pda: Option<AccountInfo<'info>>,
}

impl<'info> CheckContributions<'info> {
    pub fn check_contributions(&self, _bumps: &CheckContributionsBumps, spl_interface_bump: u8) -> Result<()> {
        let vault_balance = get_token_account_balance(&self.vault.to_account_info())?;
        require!(
            vault_balance >= self.fundraiser.amount_to_raise,
            FundraiserError::TargetNotMet
        );

        let authority_seeds: &[&[u8]] = &[
            AUTH_SEED,
            &[self.fundraiser.auth_bump],
        ];

        let decimals = self.mint_to_raise.decimals;

        let mut cpi = TransferInterfaceCpi::new(
            vault_balance,
            decimals,
            self.vault.to_account_info(),
            self.maker_ata.to_account_info(),
            self.authority.to_account_info(),
            self.fee_payer.to_account_info(),
            self.light_token_cpi_authority.to_account_info(),
            self.system_program.to_account_info(),
        );
        if self.spl_interface_pda.is_some() {
            cpi = cpi.with_spl_interface(
                Some(self.mint_to_raise.to_account_info()),
                Some(self.token_program.to_account_info()),
                self.spl_interface_pda.as_ref().map(|a| a.to_account_info()),
                Some(spl_interface_bump),
            )?;
        }
        cpi.invoke_signed(&[authority_seeds])?;

        Ok(())
    }
}
