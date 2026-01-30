#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::CloseAccountCpi;

declare_id!("GXLCuNhnkRVp596eCdbNsZ9ua1ePbKbb344VKS7V3zQQ");

#[program]
pub mod light_token_anchor_close {
    use super::*;

    pub fn close_account(ctx: Context<CloseAccountAccounts>) -> Result<()> {
        CloseAccountCpi {
            token_program: ctx.accounts.light_token_program.to_account_info(),
            account: ctx.accounts.account.to_account_info(),
            destination: ctx.accounts.destination.to_account_info(),
            owner: ctx.accounts.owner.to_account_info(),
            rent_sponsor: ctx.accounts.rent_sponsor.to_account_info(),
        }
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CloseAccountAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub account: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    pub owner: Signer<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
}
