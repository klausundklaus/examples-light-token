use anchor_lang::prelude::*;
use light_sdk::interface::CreateAccountsProof;
use light_token::anchor::LightAccounts;
use light_token::instruction::{MintToCpi, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintTokenParams {
    pub amount: u64,
    /// Proof for creating the ATA if needed
    pub create_accounts_proof: CreateAccountsProof,
    /// Bump for deriving the ATA address (from light_token::instruction::derive_token_ata)
    pub ata_bump: u8,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: MintTokenParams)]
pub struct MintTo<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// The mint authority (must be writable since light-token MintTo marks it as writable in CPI)
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    /// CHECK: The Light mint account
    #[account(mut)]
    pub mint: AccountInfo<'info>,

    /// The recipient (owner of the ATA)
    /// CHECK: This is the owner of the destination token account
    pub recipient: AccountInfo<'info>,

    /// CHECK: The destination token account (ATA) - created by light_account macro if needed
    #[account(mut)]
    #[light_account(init,
        associated_token::authority = recipient,
        associated_token::mint = mint,
        associated_token::bump = params.ata_bump
    )]
    pub destination: UncheckedAccount<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Light token config
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: Light token rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,
}

/// Mints tokens to a destination token account, creating the ATA if needed.
/// The ATA is created automatically by the #[light_account(init, associated_token, ...)] macro.
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
