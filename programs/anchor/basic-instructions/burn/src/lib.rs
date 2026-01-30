#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::BurnCpi;

declare_id!("2TXVn8AqjfyeJvmFBD3kHJmh6fWkC4HNB5T76BmLKV5c");

#[program]
pub mod light_token_anchor_burn {
    use super::*;

    pub fn burn(ctx: Context<BurnAccounts>, amount: u64) -> Result<()> {
        BurnCpi {
            source: ctx.accounts.source.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            amount,
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
pub struct BurnAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub source: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
