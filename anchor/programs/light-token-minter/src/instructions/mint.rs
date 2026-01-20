use anchor_lang::prelude::*;
use light_token::instruction::MintToCpi;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintTokenParams {
    pub amount: u64,
}

#[derive(Accounts)]
#[instruction(params: MintTokenParams)]
pub struct MintTo<'info> {
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    /// CHECK: The Light mint account
    #[account(mut)]
    pub mint: AccountInfo<'info>,

    /// CHECK: The destination token account
    #[account(mut)]
    pub destination: AccountInfo<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

/// Mints tokens to a destination token account.
pub fn mint_token<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintTo<'info>>,
    params: &MintTokenParams,
) -> Result<()> {
    MintToCpi {
        mint: ctx.accounts.mint.to_account_info(),
        destination: ctx.accounts.destination.to_account_info(),
        amount: params.amount,
        authority: ctx.accounts.mint_authority.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        max_top_up: None,
    }
    .invoke()?;

    msg!(
        "Minted {} tokens to {}",
        params.amount,
        ctx.accounts.destination.key()
    );
    Ok(())
}
