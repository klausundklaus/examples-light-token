use anchor_lang::prelude::*;
use light_token::instruction::{MintToCpi, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintTokenParams {
    pub amount: u64,
}

#[derive(Accounts)]
pub struct MintTo<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Must be writable because light-token MintTo marks authority as writable in CPI.
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    /// CHECK: Light mint account.
    #[account(mut)]
    pub mint: AccountInfo<'info>,

    /// CHECK: Recipient and owner of the destination token account.
    pub recipient: AccountInfo<'info>,

    /// CHECK: Destination Light ATA, derived client-side via derive_token_ata.
    /// Created by the Light Token Program during MintTo CPI if it does not exist.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,

    /// CHECK: Light token program.
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Light token compressible config.
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: Light token rent sponsor.
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority.
    pub light_token_cpi_authority: AccountInfo<'info>,
}

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
        fee_payer: Some(ctx.accounts.fee_payer.to_account_info()),
    }
    .invoke()?;

    msg!(
        "Minted {} tokens to {} (owner: {})",
        params.amount,
        ctx.accounts.destination.key(),
        ctx.accounts.recipient.key()
    );
    Ok(())
}
