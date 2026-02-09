//! Swap instruction account parsing.

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::constants::*;
use crate::error::SwapError;

/// Parameters for the swap instruction.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct SwapParams {
    /// Amount of input tokens to swap.
    pub amount_in: u64,
    /// Minimum amount of output tokens expected (slippage protection).
    pub minimum_amount_out: u64,
    /// Direction of swap: true = A to B, false = B to A.
    pub a_to_b: bool,
    /// Pool authority bump.
    pub authority_bump: u8,
}

/// Accounts for the swap instruction.
pub struct SwapAccounts<'a> {
    /// User performing the swap (signer).
    pub user: &'a AccountInfo,
    /// Pool state account.
    pub pool: &'a AccountInfo,
    /// Pool authority PDA.
    pub pool_authority: &'a AccountInfo,
    /// Token A mint.
    pub mint_a: &'a AccountInfo,
    /// Token B mint.
    pub mint_b: &'a AccountInfo,
    /// Token A vault.
    pub vault_a: &'a AccountInfo,
    /// Token B vault.
    pub vault_b: &'a AccountInfo,
    /// User's token A account.
    pub user_token_a: &'a AccountInfo,
    /// User's token B account.
    pub user_token_b: &'a AccountInfo,
    /// Light token program for CPI.
    pub light_token_program: &'a AccountInfo,
    /// Light token CPI authority.
    pub light_token_cpi_authority: &'a AccountInfo,
    /// System program.
    pub system_program: &'a AccountInfo,
}

impl<'a> SwapAccounts<'a> {
    /// Number of fixed accounts for this instruction.
    pub const FIXED_LEN: usize = 12;

    /// Parse accounts from the account info slice.
    pub fn parse(accounts: &'a [AccountInfo], params: &SwapParams) -> Result<Self, ProgramError> {
        if accounts.len() < Self::FIXED_LEN {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let user = &accounts[0];
        let pool = &accounts[1];
        let pool_authority = &accounts[2];
        let mint_a = &accounts[3];
        let mint_b = &accounts[4];
        let vault_a = &accounts[5];
        let vault_b = &accounts[6];
        let user_token_a = &accounts[7];
        let user_token_b = &accounts[8];
        let light_token_program = &accounts[9];
        let light_token_cpi_authority = &accounts[10];
        let system_program = &accounts[11];

        // Validate user is signer
        if !user.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate global pool authority PDA
        {
            let seeds: &[&[u8]] = &[POOL_AUTHORITY_SEED];
            let (expected_pda, expected_bump) =
                pinocchio::pubkey::find_program_address(seeds, &crate::ID);
            if pool_authority.key() != &expected_pda {
                return Err(SwapError::InvalidAuthoritySeeds.into());
            }
            if expected_bump != params.authority_bump {
                return Err(SwapError::InvalidAuthoritySeeds.into());
            }
        }

        Ok(Self {
            user,
            pool,
            pool_authority,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            user_token_a,
            user_token_b,
            light_token_program,
            light_token_cpi_authority,
            system_program,
        })
    }

    /// Get input/output accounts based on swap direction.
    /// Returns (vault_in, vault_out, user_in, user_out, mint_in, mint_out).
    pub fn get_directional_accounts(
        &self,
        a_to_b: bool,
    ) -> (
        &AccountInfo,
        &AccountInfo,
        &AccountInfo,
        &AccountInfo,
        &AccountInfo,
        &AccountInfo,
    ) {
        if a_to_b {
            (
                self.vault_a,
                self.vault_b,
                self.user_token_a,
                self.user_token_b,
                self.mint_a,
                self.mint_b,
            )
        } else {
            (
                self.vault_b,
                self.vault_a,
                self.user_token_b,
                self.user_token_a,
                self.mint_b,
                self.mint_a,
            )
        }
    }
}
