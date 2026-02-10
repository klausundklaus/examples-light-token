#![allow(dead_code)]

//! SwapSdk implementing LightProgramInterface trait.

use borsh::BorshDeserialize;
use light_account::token::Token;
use light_client::interface::{
    AccountInterface, AccountSpec, ColdContext, LightProgramInterface, PdaSpec,
};
use pinocchio_swap::{LightAccountVariant, PoolState, PoolStateSeeds, VaultSeeds};
use solana_pubkey::Pubkey;

pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(pinocchio_swap::ID);

/// Instructions supported by the swap program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapInstruction {
    Initialize,
    Swap,
}

/// Error type for SDK operations.
#[derive(Debug, Clone)]
pub enum SwapSdkError {
    ParseError(String),
}

impl std::fmt::Display for SwapSdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for SwapSdkError {}

/// Flat SDK struct. All fields populated at construction from pool state data.
#[derive(Debug)]
pub struct SwapSdk {
    pub pool_state_pubkey: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub pool_authority: Pubkey,
}

impl SwapSdk {
    /// Construct from pool state pubkey and its account data.
    pub fn new(pool_state_pubkey: Pubkey, pool_data: &[u8]) -> Result<Self, SwapSdkError> {
        let pool = PoolState::deserialize(&mut &pool_data[8..])
            .map_err(|e| SwapSdkError::ParseError(e.to_string()))?;

        let (pool_authority, _) = Pubkey::find_program_address(
            &[pinocchio_swap::constants::POOL_AUTHORITY_SEED],
            &PROGRAM_ID,
        );

        Ok(Self {
            pool_state_pubkey,
            token_a_mint: Pubkey::new_from_array(pool.token_a_mint),
            token_b_mint: Pubkey::new_from_array(pool.token_b_mint),
            token_a_vault: Pubkey::new_from_array(pool.token_a_vault),
            token_b_vault: Pubkey::new_from_array(pool.token_b_vault),
            pool_authority,
        })
    }

    /// Convert token vault ColdContext::Token -> ColdContext::Account.
    /// Vaults are decompressed as PDAs, not as token accounts.
    fn convert_vault_interface(
        account: &AccountInterface,
    ) -> Result<AccountInterface, SwapSdkError> {
        if account.is_cold() {
            let compressed_account = match &account.cold {
                Some(ColdContext::Token(ct)) => ct.account.clone(),
                Some(ColdContext::Account(ca)) => ca.clone(),
                Some(ColdContext::Mint(_)) => {
                    return Err(SwapSdkError::ParseError(
                        "unexpected Mint cold context for vault".to_string(),
                    ))
                }
                None => {
                    return Err(SwapSdkError::ParseError(
                        "missing cold context for vault".to_string(),
                    ))
                }
            };
            Ok(AccountInterface {
                key: account.key,
                account: account.account.clone(),
                cold: Some(ColdContext::Account(compressed_account)),
            })
        } else {
            Ok(account.clone())
        }
    }
}

impl LightProgramInterface for SwapSdk {
    type Variant = LightAccountVariant;
    type Instruction = SwapInstruction;

    fn program_id() -> Pubkey {
        PROGRAM_ID
    }

    fn instruction_accounts(&self, ix: &Self::Instruction) -> Vec<Pubkey> {
        match ix {
            SwapInstruction::Swap => vec![
                self.pool_state_pubkey,
                self.token_a_vault,
                self.token_b_vault,
                self.token_a_mint,
                self.token_b_mint,
            ],
            SwapInstruction::Initialize => vec![
                self.pool_state_pubkey,
                self.token_a_vault,
                self.token_b_vault,
                self.token_a_mint,
                self.token_b_mint,
            ],
        }
    }

    fn load_specs(
        &self,
        cold_accounts: &[AccountInterface],
    ) -> Result<Vec<AccountSpec<Self::Variant>>, Box<dyn std::error::Error>> {
        let mut specs = Vec::new();
        for account in cold_accounts {
            if account.key == self.pool_state_pubkey {
                let pool = PoolState::deserialize(&mut &account.data()[8..])
                    .map_err(|e| SwapSdkError::ParseError(e.to_string()))?;
                let variant = LightAccountVariant::PoolState {
                    seeds: PoolStateSeeds {
                        mint_a: pool.token_a_mint,
                        mint_b: pool.token_b_mint,
                    },
                    data: pool,
                };
                specs.push(AccountSpec::Pda(PdaSpec::new(
                    account.clone(),
                    variant,
                    PROGRAM_ID,
                )));
            } else if account.key == self.token_a_vault {
                let token: Token = Token::deserialize(&mut &account.data()[..])
                    .map_err(|e| SwapSdkError::ParseError(e.to_string()))?;
                let variant =
                    LightAccountVariant::Vault(light_account::token::TokenDataWithSeeds {
                        seeds: VaultSeeds {
                            pool: self.pool_state_pubkey.to_bytes(),
                            mint: self.token_a_mint.to_bytes(),
                        },
                        token_data: token,
                    });
                let interface = Self::convert_vault_interface(account)?;
                specs.push(AccountSpec::Pda(PdaSpec::new(
                    interface, variant, PROGRAM_ID,
                )));
            } else if account.key == self.token_b_vault {
                let token: Token = Token::deserialize(&mut &account.data()[..])
                    .map_err(|e| SwapSdkError::ParseError(e.to_string()))?;
                let variant =
                    LightAccountVariant::Vault(light_account::token::TokenDataWithSeeds {
                        seeds: VaultSeeds {
                            pool: self.pool_state_pubkey.to_bytes(),
                            mint: self.token_b_mint.to_bytes(),
                        },
                        token_data: token,
                    });
                let interface = Self::convert_vault_interface(account)?;
                specs.push(AccountSpec::Pda(PdaSpec::new(
                    interface, variant, PROGRAM_ID,
                )));
            } else if account.key == self.token_a_mint || account.key == self.token_b_mint {
                specs.push(AccountSpec::Mint(account.clone()));
            }
        }
        Ok(specs)
    }
}
