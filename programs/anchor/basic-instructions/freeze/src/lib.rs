#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::FreezeCpi;

declare_id!("JBMzMJX4sqCQfNVbosP2oqP1KZ5ZDWiwYTrupk687qXZ");

#[program]
pub mod light_token_anchor_freeze {
    use super::*;

    pub fn freeze(ctx: Context<FreezeAccounts>) -> Result<()> {
        FreezeCpi {
            token_account: ctx.accounts.token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            freeze_authority: ctx.accounts.freeze_authority.to_account_info(),
        }
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct FreezeAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    pub mint: AccountInfo<'info>,
    pub freeze_authority: Signer<'info>,
}
