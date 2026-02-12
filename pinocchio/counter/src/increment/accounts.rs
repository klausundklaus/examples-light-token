//! Increment instruction account parsing.

use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::constants::*;

/// Accounts for the increment instruction.
///
/// Layout:
/// [0] owner (signer)
/// [1] counter (writable) — validated PDA + program owner check
pub struct IncrementAccounts<'a> {
    /// Counter owner (signer).
    pub owner: &'a AccountInfo,
    /// Counter PDA (writable).
    pub counter: &'a AccountInfo,
}

impl<'a> IncrementAccounts<'a> {
    /// Number of fixed accounts for this instruction.
    pub const FIXED_LEN: usize = 2;

    /// Parse accounts from the account info slice.
    pub fn parse(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        if accounts.len() < Self::FIXED_LEN {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let owner = &accounts[0];
        let counter = &accounts[1];

        // Validate owner is signer
        if !owner.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate counter PDA
        {
            let owner_key = owner.key();
            let seeds: &[&[u8]] = &[COUNTER_SEED, owner_key.as_ref()];
            let (expected_pda, _) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if counter.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // Validate counter is owned by this program
        if counter.owner() != &crate::ID {
            return Err(ProgramError::IllegalOwner);
        }

        Ok(Self { owner, counter })
    }
}
