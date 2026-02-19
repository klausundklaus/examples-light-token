#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::ApproveCpi;

declare_id!("37XmzKqSG2VD1ZBvzyfbt1HN1mT1bqVAmfzX2ziB3KT1");

#[program]
pub mod light_token_anchor_approve {
    use super::*;

    pub fn approve(ctx: Context<ApproveAccounts>, amount: u64) -> Result<()> {
        ApproveCpi {
            token_account: ctx.accounts.token_account.to_account_info(),
            delegate: ctx.accounts.delegate.to_account_info(),
            owner: ctx.accounts.owner.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            amount,
            max_top_up: None,
        }
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ApproveAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    pub delegate: AccountInfo<'info>,
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}
