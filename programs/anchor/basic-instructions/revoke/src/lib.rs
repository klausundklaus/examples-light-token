#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::RevokeCpi;

declare_id!("G3ph4MK5qaSdxYnfxToETg31AHEMMqVhPuMRgBhk38tQ");

#[program]
pub mod light_token_anchor_revoke {
    use super::*;

    pub fn revoke(ctx: Context<RevokeAccounts>) -> Result<()> {
        RevokeCpi {
            token_account: ctx.accounts.token_account.to_account_info(),
            owner: ctx.accounts.owner.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            max_top_up: None,
        }
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RevokeAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}
