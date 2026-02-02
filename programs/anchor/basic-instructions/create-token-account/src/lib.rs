#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::CreateTokenAccountCpi;

declare_id!("zXK1CnWj4WFfFHCArxxr4sh3Qqx2p3oui8ahqpjArgS");

#[program]
pub mod light_token_anchor_create_token_account {
    use super::*;

    pub fn create_token_account(ctx: Context<CreateTokenAccountAccounts>, owner: Pubkey) -> Result<()> {
        CreateTokenAccountCpi {
            payer: ctx.accounts.payer.to_account_info(),
            account: ctx.accounts.account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner,
        }
        .rent_free(
            ctx.accounts.compressible_config.to_account_info(),
            ctx.accounts.rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &ctx.accounts.light_token_program.key(),
        )
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateTokenAccountAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub account: Signer<'info>,
    /// CHECK: Validated by light-token CPI
    pub mint: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    pub compressible_config: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
}
