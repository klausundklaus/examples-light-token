//! Initialize instruction account parsing.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CreateAccountsProof, LightAccount, LightDiscriminator};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::Sysvar,
};

use crate::constants::*;
use crate::state::PoolState;

/// Parameters for the initialize instruction.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct InitializeParams {
    /// Proof data for creating compressed accounts.
    pub create_accounts_proof: CreateAccountsProof,
    /// Fee in basis points (e.g., 30 = 0.3%).
    pub fee_bps: u16,
    /// Mint signer A bump.
    pub mint_signer_a_bump: u8,
    /// Mint signer B bump.
    pub mint_signer_b_bump: u8,
    /// Pool bump.
    pub pool_bump: u8,
    /// Vault A bump.
    pub vault_a_bump: u8,
    /// Vault B bump.
    pub vault_b_bump: u8,
}

/// Accounts for the initialize instruction.
pub struct InitializeAccounts<'a> {
    /// Payer/creator of the pool (signer).
    pub payer: &'a AccountInfo,
    /// Authority for mints (signer).
    pub authority: &'a AccountInfo,
    /// Mint signers slice (mint_signer_a, mint_signer_b).
    pub mint_signers: &'a [AccountInfo],
    /// Mints slice (mint_a, mint_b).
    pub mints: &'a [AccountInfo],
    /// Pool state account (PDA).
    pub pool: &'a AccountInfo,
    /// Pool authority PDA.
    pub pool_authority: &'a AccountInfo,
    /// Vaults slice (vault_a, vault_b).
    pub vaults: &'a [AccountInfo],
    /// Compressible config account (swap program's config for pool PDA).
    pub compressible_config: &'a AccountInfo,
    /// Rent sponsor for pool PDA.
    pub rent_sponsor: &'a AccountInfo,
    /// Light token config (LIGHT_TOKEN_CONFIG for mint creation).
    pub light_token_config: &'a AccountInfo,
    /// Light token rent sponsor (for mint creation).
    pub light_token_rent_sponsor: &'a AccountInfo,
    /// Light token program.
    pub light_token_program: &'a AccountInfo,
    /// CPI authority.
    pub cpi_authority: &'a AccountInfo,
    /// System program.
    pub system_program: &'a AccountInfo,
}

impl<'a> InitializeAccounts<'a> {
    /// Number of fixed accounts for this instruction.
    /// payer, authority, mint_signer_a, mint_signer_b, mint_a, mint_b,
    /// pool, pool_authority, vault_a, vault_b,
    /// compressible_config, rent_sponsor, light_token_config, light_token_rent_sponsor,
    /// light_token_program, cpi_authority, system_program
    pub const FIXED_LEN: usize = 17;

    /// Parse accounts from the account info slice.
    pub fn parse(
        accounts: &'a [AccountInfo],
        params: &InitializeParams,
    ) -> Result<Self, ProgramError> {
        if accounts.len() < Self::FIXED_LEN {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let payer = &accounts[0];
        let authority = &accounts[1];
        let mint_signers = &accounts[2..4]; // mint_signer_a, mint_signer_b
        let mints = &accounts[4..6]; // mint_a, mint_b
        let pool = &accounts[6];
        let pool_authority = &accounts[7];
        let vaults = &accounts[8..10]; // vault_a, vault_b
        let compressible_config = &accounts[10];
        let rent_sponsor = &accounts[11];
        let light_token_config = &accounts[12];
        let light_token_rent_sponsor = &accounts[13];
        let light_token_program = &accounts[14];
        let cpi_authority = &accounts[15];
        let system_program = &accounts[16];

        // Validate signers
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !authority.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate mint_signer_a PDA
        {
            let authority_key = authority.key();
            let seeds: &[&[u8]] = &[MINT_A_SEED, authority_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if mint_signers[0].key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.mint_signer_a_bump {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // Validate mint_signer_b PDA
        {
            let authority_key = authority.key();
            let seeds: &[&[u8]] = &[MINT_B_SEED, authority_key];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if mint_signers[1].key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if expected_bump != params.mint_signer_b_bump {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // Validate and create pool PDA
        {
            let mint_a_key = mints[0].key();
            let mint_b_key = mints[1].key();
            let seeds: &[&[u8]] = &[POOL_SEED, mint_a_key.as_ref(), mint_b_key.as_ref()];
            let (expected_pda, bump) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if pool.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
            if bump != params.pool_bump {
                return Err(ProgramError::InvalidSeeds);
            }

            // Create pool account
            let space = 8 + PoolState::INIT_SPACE;
            let rent = pinocchio::sysvars::rent::Rent::get()
                .map_err(|_| ProgramError::UnsupportedSysvar)?;
            let lamports = rent.minimum_balance(space);

            let bump_bytes = [bump];
            let seed_array = [
                Seed::from(POOL_SEED),
                Seed::from(mint_a_key.as_ref()),
                Seed::from(mint_b_key.as_ref()),
                Seed::from(bump_bytes.as_ref()),
            ];
            let signer = Signer::from(&seed_array);
            pinocchio_system::instructions::CreateAccount {
                from: payer,
                to: pool,
                lamports,
                space: space as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[signer])?;

            // Write discriminator to first 8 bytes
            let mut data = pool
                .try_borrow_mut_data()
                .map_err(|_| ProgramError::AccountBorrowFailed)?;
            data[..8].copy_from_slice(&PoolState::LIGHT_DISCRIMINATOR);
        }

        // Validate global pool authority PDA
        {
            let seeds: &[&[u8]] = &[POOL_AUTHORITY_SEED];
            let (expected_pda, _) = pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if pool_authority.key() != &expected_pda {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        Ok(Self {
            payer,
            authority,
            mint_signers,
            mints,
            pool,
            pool_authority,
            vaults,
            compressible_config,
            rent_sponsor,
            light_token_config,
            light_token_rent_sponsor,
            light_token_program,
            cpi_authority,
            system_program,
        })
    }

    /// Get mint A account.
    pub fn mint_a(&self) -> &AccountInfo {
        &self.mints[0]
    }

    /// Get mint B account.
    pub fn mint_b(&self) -> &AccountInfo {
        &self.mints[1]
    }

    /// Get vault A account.
    pub fn vault_a(&self) -> &AccountInfo {
        &self.vaults[0]
    }

    /// Get vault B account.
    pub fn vault_b(&self) -> &AccountInfo {
        &self.vaults[1]
    }

    /// Get mint signer A account.
    pub fn mint_signer_a(&self) -> &AccountInfo {
        &self.mint_signers[0]
    }

    /// Get mint signer B account.
    pub fn mint_signer_b(&self) -> &AccountInfo {
        &self.mint_signers[1]
    }
}
