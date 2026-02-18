use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::{TransferCheckedCpi, TransferInterfaceCpi, LIGHT_TOKEN_RENT_SPONSOR};
use light_token::utils::get_token_account_balance;

use crate::constants::{AUTH_SEED, VAULT_SEED};
use crate::state::{Contributor, Fundraiser};
use crate::{FundraiserError, SECONDS_TO_DAYS};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub contributor: Signer<'info>,

    pub maker: SystemAccount<'info>,

    /// CHECK: Authority PDA — signs vault operations. Writable for Light Token CPI.
    #[account(seeds = [AUTH_SEED], bump)]
    pub authority: UncheckedAccount<'info>,

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
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: SPL interface PDA for mint (token pool holding SPL tokens)
    /// Derived by light-token program: ["pool", mint]
    #[account(mut)]
    pub spl_interface_pda: UncheckedAccount<'info>,
}

impl<'info> Refund<'info> {
    pub fn refund(&mut self, _bumps: &RefundBumps, spl_interface_bump: u8) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        require!(
            self.fundraiser.duration
                < (current_time.checked_sub(self.fundraiser.time_started)
                    .ok_or(FundraiserError::CalculationOverflow)?
                    .checked_div(SECONDS_TO_DAYS)
                    .ok_or(FundraiserError::CalculationOverflow)?) as u16,
            FundraiserError::FundraiserNotEnded
        );

        let vault_balance = get_token_account_balance(&self.vault.to_account_info())
            .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
        require!(
            vault_balance < self.fundraiser.amount_to_raise,
            FundraiserError::TargetMet
        );

        let authority_seeds: &[&[u8]] = &[
            AUTH_SEED,
            &[self.fundraiser.auth_bump],
        ];

        let decimals = self.mint_to_raise.decimals;
        let refund_amount = self.contributor_account.amount;

        if self.spl_interface_pda.key() != Pubkey::default() {
            let cpi = TransferInterfaceCpi::new(
                refund_amount,
                decimals,
                self.vault.to_account_info(),
                self.contributor_ata.to_account_info(),
                self.authority.to_account_info(),
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

            cpi.invoke_signed(&[authority_seeds])
             ?;
        } else {
            TransferCheckedCpi {
                source: self.vault.to_account_info(),
                mint: self.mint_to_raise.to_account_info(),
                destination: self.contributor_ata.to_account_info(),
                amount: refund_amount,
                decimals,
                authority: self.authority.to_account_info(),
                system_program: self.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: Some(self.contributor.to_account_info()),
            }
            .invoke_signed(&[authority_seeds])
     ?;
        }

        self.fundraiser.current_amount = self.fundraiser.current_amount
            .checked_sub(refund_amount)
            .ok_or(FundraiserError::CalculationOverflow)?;

        Ok(())
    }
}
