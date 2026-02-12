//! Constants for the counter program including seeds and discriminators.

/// Seed for counter state PDA.
pub const COUNTER_SEED: &[u8] = b"counter";

/// Instruction discriminators (Anchor-compatible: sha256("global:{name}")[..8]).
pub mod discriminators {
    /// Create counter instruction.
    pub const CREATE_COUNTER: [u8; 8] = [174, 255, 78, 222, 78, 250, 200, 80];

    /// Increment counter instruction.
    pub const INCREMENT: [u8; 8] = [11, 18, 104, 9, 104, 174, 59, 33];
}
