use anchor_lang::prelude::*;
use light_sdk::LightDiscriminator;
use light_token::anchor::{CompressionInfo, LightAccount};

/// The Offer account stores details about a token swap offer.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct Offer {
    /// Compression info for Light Protocol integration.
    pub compression_info: CompressionInfo,
    /// Unique identifier for the offer.
    pub id: u64,
    /// The maker (creator) of this offer.
    pub maker: Pubkey,
    /// The mint of the token being offered.
    pub token_mint_a: Pubkey,
    /// The mint of the token wanted in exchange.
    pub token_mint_b: Pubkey,
    /// The amount of token_b the maker wants to receive.
    pub token_b_wanted_amount: u64,
    /// PDA bump seed for the authority.
    pub auth_bump: u8,
}
