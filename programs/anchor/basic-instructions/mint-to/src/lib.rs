#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::MintToCpi;

declare_id!("8bXEVmHLtAVqDLJp1dYWAZ61WQmqQKoTQ8LpPbRoUDCp");

#[program]
pub mod light_token_anchor_mint_to {
    use super::*;

    pub fn mint_to(ctx: Context<MintToAccounts>, amount: u64) -> Result<()> {
        MintToCpi {
            mint: ctx.accounts.mint.to_account_info(),
            destination: ctx.accounts.destination.to_account_info(),
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
pub struct MintToAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
