#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_token::instruction::CreateAssociatedAccountCpi;

declare_id!("35MukgdfpNUbPMhTmEk63ECV8vjgpNVFRH9nP8ovMN58");

#[program]
pub mod light_token_anchor_create_associated_token_account {
    use super::*;

    pub fn create_associated_token_account(ctx: Context<CreateAssociatedTokenAccountAccounts>, idempotent: bool) -> Result<()> {
        let cpi = CreateAssociatedAccountCpi {
            payer: ctx.accounts.payer.to_account_info(),
            owner: ctx.accounts.owner.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            ata: ctx.accounts.associated_token_account.to_account_info(),
        };

        if idempotent {
            cpi.idempotent().rent_free(
                ctx.accounts.compressible_config.to_account_info(),
                ctx.accounts.rent_sponsor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            )
        } else {
            cpi.rent_free(
                ctx.accounts.compressible_config.to_account_info(),
                ctx.accounts.rent_sponsor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            )
        }
        .invoke()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateAssociatedTokenAccountAccounts<'info> {
    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    pub owner: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub associated_token_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Validated by light-token CPI
    pub compressible_config: AccountInfo<'info>,
    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
}
