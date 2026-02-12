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

    /// Increment the counter. Standard Anchor — no Light-specific changes.
    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        ctx.accounts.counter.count = ctx.accounts.counter.count.checked_add(1).unwrap();
        Ok(())
    }

    /// Close the counter. Standard Anchor — no Light-specific changes.
    pub fn close_counter(_ctx: Context<CloseCounter>) -> Result<()> {
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

/// Standard Anchor
#[derive(Accounts)]
pub struct Increment<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [COUNTER_SEED, owner.key().as_ref()],
        bump,
        has_one = owner,
    )]
    pub counter: Account<'info, Counter>,
}

/// Standard Anchor close
#[derive(Accounts)]
pub struct CloseCounter<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub owner: Signer<'info>,

    #[account(
        mut,
        close = fee_payer,
        seeds = [COUNTER_SEED, owner.key().as_ref()],
        bump,
        has_one = owner,
    )]
    pub counter: Account<'info, Counter>,
}
