use anchor_lang::prelude::*;
use light_account::CreateAccountsProof;
use light_account::LightAccounts;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

use crate::MINT_SIGNER_SEED;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMintParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub decimals: u8,
    pub mint_signer_bump: u8,
    pub token_name: String,
    pub token_symbol: String,
    pub token_uri: String,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: PDA derived from authority, used as mint signer seed.
    #[account(
        seeds = [MINT_SIGNER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    /// CHECK: Initialized by light_account macro.
    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = params.decimals,
        mint::seeds = &[MINT_SIGNER_SEED, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump,
        mint::name = params.token_name.clone().into_bytes(),
        mint::symbol = params.token_symbol.clone().into_bytes(),
        mint::uri = params.token_uri.clone().into_bytes(),
        mint::update_authority = authority
    )]
    pub light_mint: UncheckedAccount<'info>,

    /// CHECK: Config for light mint creation.
    pub compression_config: AccountInfo<'info>,

    /// CHECK: Light token compressible config.
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: Light token rent sponsor.
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program.
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: Light token CPI authority.
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_token<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateMint<'info>>,
    params: &CreateMintParams,
) -> Result<()> {
    msg!("Light mint created with metadata");
    msg!("Name: {}", params.token_name);
    msg!("Symbol: {}", params.token_symbol);
    msg!("URI: {}", params.token_uri);
    msg!("Mint signer: {}", ctx.accounts.mint_signer.key());
    msg!("Mint authority: {}", ctx.accounts.authority.key());
    Ok(())
}
