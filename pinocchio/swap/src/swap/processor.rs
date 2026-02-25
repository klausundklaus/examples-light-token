//! Swap instruction processor.

use light_token_pinocchio::instruction::TransferCpi;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use super::accounts::{SwapAccounts, SwapParams};
use crate::constants::*;
use crate::error::SwapError;
use crate::state::PoolState;

/// Offset to amount field in token account data.
/// Layout: mint (32 bytes) + owner (32 bytes) + amount (8 bytes)
const TOKEN_AMOUNT_OFFSET: usize = 64;

/// Read token balance from account data.
fn read_token_balance(account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = account
        .try_borrow_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    if data.len() < TOKEN_AMOUNT_OFFSET + 8 {
        return Err(SwapError::InvalidPoolState.into());
    }

    let amount_bytes: [u8; 8] = data[TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8]
        .try_into()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(u64::from_le_bytes(amount_bytes))
}

/// Process the swap instruction.
pub fn process(
    ctx: &SwapAccounts,
    params: &SwapParams,
    _remaining_accounts: &[AccountInfo],
) -> Result<(), SwapError> {
    // 1. Validate amount is non-zero
    if params.amount_in == 0 {
        return Err(SwapError::ZeroAmount);
    }

    // 2. Load and validate pool state
    let pool_data = ctx
        .pool
        .try_borrow_data()
        .map_err(|_| SwapError::InvalidPoolState)?;

    // Skip 8-byte discriminator
    if pool_data.len() < 8 + core::mem::size_of::<PoolState>() {
        return Err(SwapError::InvalidPoolState);
    }

    let pool_state: &PoolState =
        bytemuck::from_bytes(&pool_data[8..8 + core::mem::size_of::<PoolState>()]);

    // 3. Verify vaults match pool state
    let vault_a_key = ctx.vault_a.key();
    let vault_b_key = ctx.vault_b.key();
    if vault_a_key != &pool_state.token_a_vault || vault_b_key != &pool_state.token_b_vault {
        return Err(SwapError::InvalidVaultSeeds);
    }

    // 4. Verify mints match pool state
    let mint_a_key = ctx.mint_a.key();
    let mint_b_key = ctx.mint_b.key();
    if mint_a_key != &pool_state.token_a_mint || mint_b_key != &pool_state.token_b_mint {
        return Err(SwapError::InvalidMint);
    }

    // 5. Get directional accounts
    let (vault_in, vault_out, user_in, user_out, _mint_in, _mint_out) =
        ctx.get_directional_accounts(params.a_to_b);

    // 6. Read vault balances
    let reserve_in = read_token_balance(vault_in).map_err(|_| SwapError::InvalidPoolState)?;
    let reserve_out = read_token_balance(vault_out).map_err(|_| SwapError::InvalidPoolState)?;

    // 7. Calculate output amount using constant product formula
    let amount_out = pool_state
        .calculate_swap_output(params.amount_in, reserve_in, reserve_out)
        .ok_or(SwapError::InsufficientLiquidity)?;

    // 8. Check slippage tolerance
    if amount_out < params.minimum_amount_out {
        return Err(SwapError::SlippageExceeded);
    }

    // Drop pool data borrow before CPIs
    drop(pool_data);

    // 9. Build global authority seeds for signing
    let authority_bump = [params.authority_bump];

    // Convert seeds to pinocchio Seed type (global authority, no pool key)
    let seeds: [Seed; 2] = [
        Seed::from(POOL_AUTHORITY_SEED),
        Seed::from(&authority_bump[..]),
    ];
    let signer = Signer::from(&seeds[..]);

    // 10. Transfer input tokens from user to vault
    TransferCpi {
        source: user_in,
        destination: vault_in,
        amount: params.amount_in,
        authority: ctx.user,
        system_program: ctx.system_program,
        fee_payer: Some(ctx.user),
    }
    .invoke()
    .map_err(|_| SwapError::InvalidTokenOwner)?;

    // 11. Transfer output tokens from vault to user (with PDA signing)
    TransferCpi {
        source: vault_out,
        destination: user_out,
        amount: amount_out,
        authority: ctx.pool_authority,
        system_program: ctx.system_program,
        fee_payer: Some(ctx.user),
    }
    .invoke_signed(&[signer])
    .map_err(|_| SwapError::InvalidTokenOwner)?;

    Ok(())
}
