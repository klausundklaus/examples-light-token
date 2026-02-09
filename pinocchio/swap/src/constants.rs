//! Constants for the swap program including seeds and discriminators.

/// Seed for pool state PDA.
pub const POOL_SEED: &[u8] = b"pool";

/// Seed for global pool authority PDA (controls all vaults).
/// Using a single global authority (like cp-swap) enables vault decompression.
pub const POOL_AUTHORITY_SEED: &[u8] = b"pool_authority";

/// Seed for pool vault token accounts.
pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";

/// Seed for mint A.
pub const MINT_A_SEED: &[u8] = b"mint_a";

/// Seed for mint B.
pub const MINT_B_SEED: &[u8] = b"mint_b";

/// Seed for user token accounts.
pub const USER_TOKEN_SEED: &[u8] = b"user_token";

/// Instruction discriminators (Anchor-compatible: sha256("global:{name}")[..8]).
pub mod discriminators {
    /// Initialize pool instruction.
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

    /// Swap instruction.
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
}

/// Default fee in basis points (0.3%).
pub const DEFAULT_FEE_BPS: u16 = 30;

/// Mint decimals for tokens.
pub const MINT_DECIMALS: u8 = 9;
