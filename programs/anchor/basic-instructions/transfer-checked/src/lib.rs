#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::TransferCheckedCpi;

declare_id!("HXmfewpozFdxhM8BayL9v5541gwoGMXTrUoip5KySs2f");

#[program]
pub mod light_token_anchor_transfer_checked {
    use super::*;

    pub fn transfer_checked(
        ctx: Context<TransferCheckedAccounts>,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        TransferCheckedCpi {
            source: ctx.accounts.source.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            destination: ctx.accounts.destination.to_account_info(),
            amount,
            decimals,
            authority: ctx.accounts.authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            max_top_up: None,
            fee_payer: None,
        }
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct TransferCheckedAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub source: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    pub mint: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
