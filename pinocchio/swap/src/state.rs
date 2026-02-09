//! Pool state for the AMM swap program.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CompressionInfo, LightDiscriminator, LightPinocchioAccount};
use pinocchio::pubkey::Pubkey;

/// Pool state containing AMM configuration and vault information.
///
/// LightPinocchioAccount generates:
/// - LightHasherSha (DataHasher + ToByteArray)
/// - LightDiscriminator
/// - LightAccount trait impl with pack/unpack
/// - PackedPoolState struct
#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    PartialEq,
    BorshSerialize,
    BorshDeserialize,
    LightPinocchioAccount,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
#[repr(C)]
pub struct PoolState {
    /// Compression metadata for Light Protocol rent-free accounts.
    pub compression_info: CompressionInfo,
    /// Authority PDA bump seed.
    pub authority_bump: u8,
    /// Padding for alignment.
    pub _padding: [u8; 7],
    /// Token A mint address.
    pub token_a_mint: Pubkey,
    /// Token B mint address.
    pub token_b_mint: Pubkey,
    /// Token A vault address.
    pub token_a_vault: Pubkey,
    /// Token B vault address.
    pub token_b_vault: Pubkey,
    /// LP token supply (for future LP token support).
    pub lp_supply: u64,
    /// Fee in basis points (e.g., 30 = 0.3%).
    pub fee_bps: u16,
    /// Padding for alignment.
    pub _padding2: [u8; 6],
    /// Admin/creator of the pool.
    pub admin: Pubkey,
}

impl PoolState {
    /// Initialize pool state with all required parameters.
    pub fn initialize(
        &mut self,
        authority_bump: u8,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        token_a_vault: Pubkey,
        token_b_vault: Pubkey,
        fee_bps: u16,
        admin: Pubkey,
    ) {
        self.authority_bump = authority_bump;
        self.token_a_mint = token_a_mint;
        self.token_b_mint = token_b_mint;
        self.token_a_vault = token_a_vault;
        self.token_b_vault = token_b_vault;
        self.lp_supply = 0;
        self.fee_bps = fee_bps;
        self.admin = admin;
    }

    /// Calculate output amount using constant product formula.
    /// delta_y = (delta_x * y * (10000 - fee)) / (x * 10000 + delta_x * (10000 - fee))
    pub fn calculate_swap_output(
        &self,
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
    ) -> Option<u64> {
        if amount_in == 0 || reserve_in == 0 || reserve_out == 0 {
            return None;
        }

        let fee_factor = 10000u128 - self.fee_bps as u128;
        let amount_in_with_fee = (amount_in as u128).checked_mul(fee_factor)?;
        let numerator = amount_in_with_fee.checked_mul(reserve_out as u128)?;
        let denominator = (reserve_in as u128)
            .checked_mul(10000)?
            .checked_add(amount_in_with_fee)?;

        let amount_out = numerator.checked_div(denominator)?;
        if amount_out > u64::MAX as u128 {
            return None;
        }
        Some(amount_out as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_product_swap() {
        let mut pool = PoolState::default();
        pool.fee_bps = 30; // 0.3% fee

        // Pool with 1000 A and 1000 B
        let reserve_a = 1000u64;
        let reserve_b = 1000u64;

        // Swap 100 A for B
        let amount_out = pool
            .calculate_swap_output(100, reserve_a, reserve_b)
            .unwrap();
        // With 0.3% fee: (100 * 9970 * 1000) / (1000 * 10000 + 100 * 9970) = 90
        assert!(amount_out > 0 && amount_out < 100);

        // Zero inputs should return None
        assert!(pool
            .calculate_swap_output(0, reserve_a, reserve_b)
            .is_none());
        assert!(pool.calculate_swap_output(100, 0, reserve_b).is_none());
        assert!(pool.calculate_swap_output(100, reserve_a, 0).is_none());
    }
}
