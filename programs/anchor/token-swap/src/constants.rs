use anchor_lang::prelude::*;

#[constant]
pub const MINIMUM_LIQUIDITY: u64 = 100;

#[constant]
pub const AUTHORITY_SEED: &[u8] = b"authority";

#[constant]
pub const LIQUIDITY_SEED: &[u8] = b"liquidity";

#[constant]
pub const POOL_ACCOUNT_A_SEED: &[u8] = b"pool_a";

#[constant]
pub const POOL_ACCOUNT_B_SEED: &[u8] = b"pool_b";

#[constant]
pub const LP_MINT_SIGNER_SEED: &[u8] = b"lp_mint_signer";
