use anchor_lang::prelude::*;
use light_sdk::LightDiscriminator;
use light_account::{CompressionInfo, LightAccount};

#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct Offer {
    pub compression_info: CompressionInfo,
    pub id: u64,
    pub maker: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_b_wanted_amount: u64,
    pub auth_bump: u8,
}
