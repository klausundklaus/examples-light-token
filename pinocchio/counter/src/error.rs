//! Custom errors for the counter program.

use pinocchio::program_error::ProgramError;

/// Counter program errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CounterError {
    /// Invalid counter PDA seeds.
    InvalidCounterSeeds = 6000,
    /// Counter owner mismatch.
    OwnerMismatch = 6001,
    /// Math overflow.
    MathOverflow = 6002,
}

impl From<CounterError> for ProgramError {
    fn from(e: CounterError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<CounterError> for u32 {
    fn from(e: CounterError) -> Self {
        e as u32
    }
}
