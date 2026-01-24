use anchor_lang::prelude::*;
use light_sdk::interface::CreateAccountsProof;
use light_token::anchor::LightAccounts;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

use crate::MINT_SIGNER_SEED;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMintParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub decimals: u8,
    pub mint_signer_bump: u8,
    /// Token name (e.g., "My Token")
    pub token_name: String,
    /// Token symbol (e.g., "MTK")
    pub token_symbol: String,
    /// Token metadata URI (e.g., "https://example.com/metadata.json")
    pub token_uri: String,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// The mint authority
    pub authority: Signer<'info>,

    /// CHECK: PDA derived from authority - used as mint signer seed
    #[account(
        seeds = [MINT_SIGNER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    /// CHECK: Initialized by light_account macro
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
    pub cmint: UncheckedAccount<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: Light token compressible config
    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    /// CHECK: Light token rent sponsor
    #[account(mut, address = RENT_SPONSOR)]
    pub rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

/// Creates a new Light compressed token mint with metadata.
/// The mint is created automatically by the #[light_account(init, mint, ...)] macro.
#[allow(unused_variables)]
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
