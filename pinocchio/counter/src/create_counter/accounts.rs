//! Create counter instruction account parsing.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CreateAccountsProof, LightAccount, LightDiscriminator};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::Sysvar,
};

use crate::constants::*;
use crate::state::CounterState;

/// Parameters for the create_counter instruction.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CreateCounterParams {
    /// Proof data for creating compressed accounts.
    pub create_accounts_proof: CreateAccountsProof,
    /// Counter PDA bump.
    pub counter_bump: u8,
}

/// Accounts for the create_counter instruction.
///
/// Layout:
/// [0] payer (signer, writable)
/// [1] owner (read-only)
/// [2] counter (writable) — PDA: [COUNTER_SEED, owner]
/// [3] compressible_config (read-only)
/// [4] system_program (read-only)
/// ... remaining_accounts (Light Protocol system accounts)
pub struct CreateCounterAccounts<'a> {
    /// Payer/creator (signer, writable).
    pub payer: &'a AccountInfo,
    /// Counter owner (read-only).
    pub owner: &'a AccountInfo,
    /// Counter PDA (writable).
    pub counter: &'a AccountInfo,
    /// Compressible config account (read-only).
    pub compressible_config: &'a AccountInfo,
    /// System program (read-only).
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateCounterAccounts<'a> {
    /// Number of fixed accounts for this instruction.
    pub const FIXED_LEN: usize = 5;

    /// Parse accounts from the account info slice.
    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &CreateCounterParams,
    ) -> Result<Self, ProgramError> {
        if accounts.len() < Self::FIXED_LEN {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let payer = &accounts[0];
        let owner = &accounts[1];
        let counter = &accounts[2];
        let compressible_config = &accounts[3];
        let system_program = &accounts[4];

        // Validate payer is signer
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate and create counter PDA
        {
            let owner_key = owner.key();
            let seeds: &[&[u8]] = &[COUNTER_SEED, owner_key.as_ref()];
            let (expected_pda, bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if counter.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if bump != params.counter_bump {
                return Err(ProgramError::InvalidSeeds);
            }

            // Create counter account
            let space = 8 + CounterState::INIT_SPACE;
            let rent = pinocchio::sysvars::rent::Rent::get()
                .map_err(|_| ProgramError::UnsupportedSysvar)?;
            let lamports = rent.minimum_balance(space);

            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(COUNTER_SEED),
                Seed::from(owner_key.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: payer,
                to: counter,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;

            // Write discriminator to first 8 bytes
            let mut data = counter
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[..8].copy_from_slice(&CounterState::LIGHT_DISCRIMINATOR);
        }

        Ok(Self {
            payer,
            owner,
            counter,
            compressible_config,
            system_program,
        })
    }
}
