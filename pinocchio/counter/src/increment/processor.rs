//! Increment instruction processor.
//!
//! Pure on-chain mutation — no Light CPI needed.

use light_account_pinocchio::LightDiscriminator;

use super::accounts::IncrementAccounts;
use crate::error::CounterError;
use crate::state::CounterState;

/// Process the increment instruction.
pub fn process(ctx: &IncrementAccounts<'_>) -> Result<(), CounterError> {
    let mut account_data = ctx
        .counter
        .try_borrow_mut_data()
        .map_err(|_| CounterError::OwnerMismatch)?;

    // Validate discriminator
    if account_data.len() < 8 + core::mem::size_of::<CounterState>() {
        return Err(CounterError::InvalidCounterSeeds);
    }
    let disc: [u8; 8] = account_data[..8].try_into().unwrap();
    if disc != CounterState::LIGHT_DISCRIMINATOR {
        return Err(CounterError::InvalidCounterSeeds);
    }

    // Zero-copy mutation
    let record_bytes = &mut account_data[8..8 + core::mem::size_of::<CounterState>()];
    let counter_state: &mut CounterState = bytemuck::from_bytes_mut(record_bytes);

    // Validate owner
    if counter_state.owner != *ctx.owner.key() {
        return Err(CounterError::OwnerMismatch);
    }

    // Increment
    counter_state.count = counter_state
        .count
        .checked_add(1)
        .ok_or(CounterError::MathOverflow)?;

    Ok(())
}
