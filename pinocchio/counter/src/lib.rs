//! Pinocchio-based counter program using Light Protocol rent-free accounts.
//!
//! Uses #[derive(LightProgramPinocchio)] to generate compress/decompress dispatch,
//! config handlers, and variant types. No Anchor dependency.

#![allow(deprecated)]

use light_account_pinocchio::{
    derive_light_cpi_signer, pubkey_array, CpiSigner, LightAccount, LightProgramPinocchio,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

pub mod constants;
pub mod create_counter;
pub mod error;
pub mod increment;
pub mod state;

pub use constants::*;
pub use state::*;

pub const ID: Pubkey = pubkey_array!("CntrPino11111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("CntrPino11111111111111111111111111111111111");

/// Program accounts enum for LightProgramPinocchio.
/// Generates: variant enums, compress/decompress dispatch, config handlers,
/// per-variant Seeds/Variant/Packed types, LightAccountVariantTrait impls,
/// size validation, seed providers, and client functions.
#[derive(LightProgramPinocchio)]
pub enum ProgramAccounts {
    /// Counter state account storing owner and count.
    /// Seeds: [COUNTER_SEED, owner]
    #[light_account(pda::seeds = [COUNTER_SEED, ctx.owner], pda::zero_copy)]
    CounterState(CounterState),
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
        discriminators::CREATE_COUNTER => process_create_counter(accounts, data),
        discriminators::INCREMENT => process_increment(accounts, data),
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

fn process_create_counter(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use create_counter::accounts::{CreateCounterAccounts, CreateCounterParams};

    let params = CreateCounterParams::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateCounterAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let ctx = CreateCounterAccounts::parse(fixed_accounts, &params)?;

    create_counter::processor::process(&ctx, &params, remaining_accounts)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_increment(accounts: &[AccountInfo], _data: &[u8]) -> Result<(), ProgramError> {
    use increment::accounts::IncrementAccounts;

    let ctx = IncrementAccounts::parse(accounts)?;

    increment::processor::process(&ctx).map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}
