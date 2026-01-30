#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
use light_sdk::{interface::CreateAccountsProof, LightDiscriminator};
use light_token::anchor::{derive_light_cpi_signer, light_program, CompressionInfo, CpiSigner, LightAccount, LightAccounts};

declare_id!("PDAm7XVHEkBvzBYDh8qF3z8NxnYQzPjGQJKcHVmMZpT");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("PDAm7XVHEkBvzBYDh8qF3z8NxnYQzPjGQJKcHVmMZpT");

pub const COUNTER_SEED: &[u8] = b"counter";

#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct Counter {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    pub count: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateCounterParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub count: u64,
}

#[light_program]
#[program]
pub mod counter {
    use super::*;

    pub fn create_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCounter<'info>>,
        params: CreateCounterParams,
    ) -> Result<()> {
        ctx.accounts.counter.owner = ctx.accounts.owner.key();
        ctx.accounts.counter.count = params.count;
        Ok(())
    }
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateCounterParams)]
pub struct CreateCounter<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Read-only, used for PDA derivation.
    pub owner: AccountInfo<'info>,

    /// CHECK: Validated by Light Protocol CPI.
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + Counter::INIT_SPACE,
        seeds = [COUNTER_SEED, owner.key().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub counter: Account<'info, Counter>,

    pub system_program: Program<'info, System>,
}
