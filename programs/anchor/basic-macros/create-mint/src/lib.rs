#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_account::{
    derive_light_cpi_signer, light_program, CreateAccountsProof, CpiSigner, LightAccounts,
};

declare_id!("HVmVqSJyMejBeUigePMSfX4aENJzCGHNxAJuT2PDMPRx");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("HVmVqSJyMejBeUigePMSfX4aENJzCGHNxAJuT2PDMPRx");

pub const MINT_SIGNER_SEED: &[u8] = b"mint_signer";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMintParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMintWithMetadataParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_bump: u8,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: PDA derived from authority
    #[account(
        seeds = [MINT_SIGNER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    /// CHECK: Initialized by light_mint CPI
    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = fee_payer,
        mint::decimals = 9,
        mint::seeds = &[MINT_SIGNER_SEED, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump
    )]
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Compression config PDA
    pub compression_config: AccountInfo<'info>,

    /// CHECK: Light Token config
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: Rent sponsor
    #[account(mut)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: Light Token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintWithMetadataParams)]
pub struct CreateMintWithMetadata<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: PDA derived from authority
    #[account(
        seeds = [MINT_SIGNER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    /// CHECK: Initialized by light_mint CPI
    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = fee_payer,
        mint::decimals = 9,
        mint::seeds = &[MINT_SIGNER_SEED, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump,
        mint::name = params.name.clone(),
        mint::symbol = params.symbol.clone(),
        mint::uri = params.uri.clone(),
        mint::update_authority = authority
    )]
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Compression config PDA
    pub compression_config: AccountInfo<'info>,

    /// CHECK: Light Token config
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: Rent sponsor
    #[account(mut)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: Light Token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[light_program]
#[program]
pub mod light_token_macro_create_mint {
    use super::*;

    #[allow(unused_variables)]
    pub fn create_mint<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateMint<'info>>,
        params: CreateMintParams,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    pub fn create_mint_with_metadata<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateMintWithMetadata<'info>>,
        params: CreateMintWithMetadataParams,
    ) -> Result<()> {
        Ok(())
    }
}
