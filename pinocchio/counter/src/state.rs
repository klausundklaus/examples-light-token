//! Counter state for the counter program.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CompressionInfo, LightDiscriminator, LightPinocchioAccount};
use pinocchio::pubkey::Pubkey;

/// Counter state storing owner and count.
///
/// LightPinocchioAccount generates:
/// - LightHasherSha (DataHasher + ToByteArray)
/// - LightDiscriminator
/// - LightAccount trait impl with pack/unpack
/// - PackedCounterState struct
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
pub struct CounterState {
    /// Compression metadata for Light Protocol rent-free accounts.
    pub compression_info: CompressionInfo,
    /// Owner of this counter.
    pub owner: Pubkey,
    /// Current count value.
    pub count: u64,
}
