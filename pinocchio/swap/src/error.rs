//! Custom errors for the swap program.

use pinocchio::program_error::ProgramError;

/// Swap program errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SwapError {
    /// Invalid pool seeds.
    InvalidPoolSeeds = 6000,
    /// Invalid authority seeds.
    InvalidAuthoritySeeds = 6001,
    /// Invalid vault seeds.
    InvalidVaultSeeds = 6002,
    /// Invalid mint seeds.
    InvalidMintSeeds = 6003,
    /// Slippage tolerance exceeded.
    SlippageExceeded = 6004,
    /// Insufficient liquidity for swap.
    InsufficientLiquidity = 6005,
    /// Invalid token account owner.
    InvalidTokenOwner = 6006,
    /// Invalid pool state.
    InvalidPoolState = 6007,
    /// Math overflow.
    MathOverflow = 6008,
    /// Zero amount not allowed.
    ZeroAmount = 6009,
    /// Invalid mint for pool.
    InvalidMint = 6010,
    /// Account not writable.
    AccountNotWritable = 6011,
    /// Missing signer.
    MissingSigner = 6012,
}

impl From<SwapError> for ProgramError {
    fn from(e: SwapError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<SwapError> for u32 {
    fn from(e: SwapError) -> Self {
        e as u32
    }
}
