//! Pinocchio-based AMM swap program using Light Protocol rent-free accounts.
//!
//! Uses #[derive(LightProgramPinocchio)] to generate compress/decompress dispatch,
//! config handlers, and variant types. No Anchor dependency.

#![allow(deprecated)]

use light_account_pinocchio::{
    derive_light_cpi_signer, pubkey_array, CpiSigner, LightAccount, LightProgramPinocchio,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

pub mod constants;
pub mod error;
pub mod init;
pub mod state;
pub mod swap;

pub use constants::*;
pub use state::*;

// Program ID - a valid base58 Solana public key (using a similar pattern to the reference)
pub const ID: Pubkey = pubkey_array!("SwapPino11111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("SwapPino11111111111111111111111111111111111");

/// Program accounts enum for LightProgramPinocchio.
/// This generates: variant enums, compress/decompress dispatch, config handlers,
/// per-variant Seeds/Variant/Packed types, LightAccountVariantTrait impls,
/// size validation, seed providers, and client functions.
#[derive(LightProgramPinocchio)]
pub enum ProgramAccounts {
    /// Pool state account storing AMM configuration.
    /// Seeds: [POOL_SEED, mint_a, mint_b]
    #[light_account(pda::seeds = [POOL_SEED, ctx.mint_a, ctx.mint_b], pda::zero_copy)]
    PoolState(PoolState),

    /// Token vault for pool (owned by pool authority PDA).
    /// Seeds: [POOL_VAULT_SEED, pool, mint], Owner seeds: [POOL_AUTHORITY_SEED]
    #[light_account(token::seeds = [POOL_VAULT_SEED, ctx.pool, ctx.mint], token::owner_seeds = [POOL_AUTHORITY_SEED])]
    Vault,

    /// User token account (associated token account style).
    #[light_account(associated_token)]
    UserToken,
}

// ============================================================================
// Entrypoint
// ============================================================================

pinocchio::entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let (disc, data) = instruction_data.split_at(8);
    let disc: [u8; 8] = disc.try_into().unwrap();

    match disc {
        discriminators::INITIALIZE => process_initialize(accounts, data),
        discriminators::SWAP => process_swap(accounts, data),
        ProgramAccounts::INITIALIZE_COMPRESSION_CONFIG => {
            ProgramAccounts::process_initialize_config(accounts, data)
        }
        ProgramAccounts::UPDATE_COMPRESSION_CONFIG => {
            ProgramAccounts::process_update_config(accounts, data)
        }
        ProgramAccounts::COMPRESS_ACCOUNTS_IDEMPOTENT => {
            ProgramAccounts::process_compress(accounts, data)
        }
        ProgramAccounts::DECOMPRESS_ACCOUNTS_IDEMPOTENT => {
            ProgramAccounts::process_decompress(accounts, data)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ============================================================================
// Instruction Handlers
// ============================================================================

fn process_initialize(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use init::accounts::{InitializeAccounts, InitializeParams};

    let params =
        InitializeParams::deserialize(&mut &data[..]).map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = InitializeAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let ctx = InitializeAccounts::parse(fixed_accounts, &params)?;

    init::processor::process(&ctx, &params, remaining_accounts)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_swap(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use swap::accounts::{SwapAccounts, SwapParams};

    let params = SwapParams::deserialize(&mut &data[..]).map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = SwapAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let ctx = SwapAccounts::parse(fixed_accounts, &params)?;

    swap::processor::process(&ctx, &params, remaining_accounts)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}
