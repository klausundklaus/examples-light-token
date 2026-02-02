#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::TransferInterfaceCpi;

declare_id!("3rb6sG4jiYNLZC8jo8kLsFHpxr2Ci8e8Hh8UmeCMZmUV");

#[program]
pub mod light_token_anchor_transfer_interface {
    use super::*;

    pub fn transfer(
        ctx: Context<TransferAccounts>,
        amount: u64,
        decimals: u8,
        spl_interface_pda_bump: Option<u8>,
    ) -> Result<()> {
        let mut transfer = TransferInterfaceCpi::new(
            amount,
            decimals,
            ctx.accounts.source.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.cpi_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        );

        if let Some(bump) = spl_interface_pda_bump {
            transfer = transfer
                .with_spl_interface(
                    ctx.accounts.mint.as_ref().map(|a| a.to_account_info()),
                    ctx.accounts
                        .spl_token_program
                        .as_ref()
                        .map(|a| a.to_account_info()),
                    ctx.accounts
                        .spl_interface_pda
                        .as_ref()
                        .map(|a| a.to_account_info()),
                    Some(bump),
                )
                .map_err(|e| ProgramError::from(e))?;
        }

        transfer.invoke().map_err(|e| ProgramError::from(e))?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct TransferAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub source: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    pub authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Validated by light-token CPI
    pub cpi_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,

    // SPL interface accounts (optional, for cross-type transfers)
    /// CHECK: Validated by light-token CPI - token mint
    pub mint: Option<AccountInfo<'info>>,
    /// CHECK: SPL Token or Token-2022 program
    pub spl_token_program: Option<AccountInfo<'info>>,
    /// CHECK: Validated by light-token CPI - pool PDA
    #[account(mut)]
    pub spl_interface_pda: Option<AccountInfo<'info>>,
}
