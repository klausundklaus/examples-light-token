#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::ThawCpi;

declare_id!("7j94EF5hSkDLf7R26bjrd8Qc6s3oLAQpcKiF3re8JYw9");

#[program]
pub mod light_token_anchor_thaw {
    use super::*;

    pub fn thaw(ctx: Context<ThawAccounts>) -> Result<()> {
        ThawCpi {
            token_account: ctx.accounts.token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            freeze_authority: ctx.accounts.freeze_authority.to_account_info(),
        }
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ThawAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    pub mint: AccountInfo<'info>,
    pub freeze_authority: Signer<'info>,
}
