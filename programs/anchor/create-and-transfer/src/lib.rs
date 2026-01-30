#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_sdk::interface::CreateAccountsProof;
use light_token::anchor::{derive_light_cpi_signer, light_program, CpiSigner, LightAccounts};
use light_token::instruction::TransferInterfaceCpi;

declare_id!("672fL1Nm191MbPoygNM9DRiG2psBELn97XUpGbU3jW7E");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("672fL1Nm191MbPoygNM9DRiG2psBELn97XUpGbU3jW7E");

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransferParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub dest_associated_token_account_bump: u8,
    pub amount: u64,
    pub decimals: u8,
}

#[light_program]
#[program]
pub mod create_and_transfer {
    use super::*;

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, Transfer<'info>>,
        params: TransferParams,
    ) -> Result<()> {
        TransferInterfaceCpi::new(
            params.amount,
            params.decimals,
            ctx.accounts.source.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.light_token_cpi_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        )
        .invoke()
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        Ok(())
    }
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: TransferParams)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: Validated by light-token CPI
    pub mint: AccountInfo<'info>,

    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub source: AccountInfo<'info>,

    /// CHECK: Validated by light-token CPI
    pub recipient: AccountInfo<'info>,

    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    #[light_account(init,
        associated_token::authority = recipient,
        associated_token::mint = mint,
        associated_token::bump = params.dest_associated_token_account_bump
    )]
    pub destination: UncheckedAccount<'info>,

    /// CHECK: Validated by light-token CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Validated by light-token CPI
    pub light_token_compressible_config: AccountInfo<'info>,

    /// CHECK: Validated by light-token CPI
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,

    /// CHECK: Validated by light-token CPI
    pub light_token_cpi_authority: AccountInfo<'info>,
}
