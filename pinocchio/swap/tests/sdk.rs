#![allow(dead_code)]

//! SwapSdk implementing LightProgramInterface trait.
//!
//! Provides:
//! - Parsing pool accounts from AccountInterface
//! - Tracking account state (hot/cold)
//! - Building AccountSpec for load instructions

use borsh::BorshDeserialize;
use light_account::LightDiscriminator;
use light_client::interface::{
    AccountInterface, AccountSpec, AccountToFetch, ColdContext, LightProgramInterface, PdaSpec,
};
use light_account::token::Token;
use pinocchio_swap::{LightAccountVariant, PoolState, PoolStateSeeds, VaultSeeds};
use solana_pubkey::Pubkey;
use std::collections::HashMap;

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
    UnknownDiscriminator([u8; 8]),
    MissingField(&'static str),
    PoolStateNotParsed,
    AccountNotFound(Pubkey),
}

impl std::fmt::Display for SwapSdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::UnknownDiscriminator(disc) => write!(f, "Unknown discriminator: {:?}", disc),
            Self::MissingField(field) => write!(f, "Missing field: {}", field),
            Self::PoolStateNotParsed => write!(f, "Pool state must be parsed first"),
            Self::AccountNotFound(key) => write!(f, "Account not found: {}", key),
        }
    }
}

impl std::error::Error for SwapSdkError {}

/// SDK for managing swap pool accounts and building decompression instructions.
#[derive(Debug, Clone)]
pub struct SwapSdk {
    /// Pool state pubkey
    pub pool_state_pubkey: Option<Pubkey>,
    /// Token A mint pubkey
    pub token_a_mint: Option<Pubkey>,
    /// Token B mint pubkey
    pub token_b_mint: Option<Pubkey>,
    /// Token A vault pubkey
    pub token_a_vault: Option<Pubkey>,
    /// Token B vault pubkey
    pub token_b_vault: Option<Pubkey>,
    /// Pool authority pubkey
    pub pool_authority: Option<Pubkey>,
    /// Cached PDA specs keyed by pubkey (includes pool_state and vaults)
    pda_specs: HashMap<Pubkey, PdaSpec<LightAccountVariant>>,
    /// Cached mint interfaces keyed by pubkey
    mint_specs: HashMap<Pubkey, AccountInterface>,
}

impl Default for SwapSdk {
    fn default() -> Self {
        Self::new()
    }
}

impl SwapSdk {
    /// Create a new empty SDK instance.
    pub fn new() -> Self {
        Self {
            pool_state_pubkey: None,
            token_a_mint: None,
            token_b_mint: None,
            token_a_vault: None,
            token_b_vault: None,
            pool_authority: None,
            pda_specs: HashMap::new(),
            mint_specs: HashMap::new(),
        }
    }

    /// Parse pool state from AccountInterface and populate SDK fields.
    fn parse_pool_state(&mut self, interface: AccountInterface) -> Result<(), SwapSdkError> {
        let data = interface.data();
        if data.len() < 8 {
            return Err(SwapSdkError::ParseError(
                "Account data too short".to_string(),
            ));
        }

        // Skip 8-byte discriminator
        let pool_state = PoolState::deserialize(&mut &data[8..])
            .map_err(|e| SwapSdkError::ParseError(e.to_string()))?;

        let pool_pubkey = interface.key;
        self.pool_state_pubkey = Some(pool_pubkey);
        self.token_a_mint = Some(Pubkey::new_from_array(pool_state.token_a_mint));
        self.token_b_mint = Some(Pubkey::new_from_array(pool_state.token_b_mint));
        self.token_a_vault = Some(Pubkey::new_from_array(pool_state.token_a_vault));
        self.token_b_vault = Some(Pubkey::new_from_array(pool_state.token_b_vault));

        // Derive global pool authority (single seed - same for all pools)
        let (pool_authority, _) = Pubkey::find_program_address(
            &[pinocchio_swap::constants::POOL_AUTHORITY_SEED],
            &PROGRAM_ID,
        );
        self.pool_authority = Some(pool_authority);

        // Create PdaSpec with variant
        let variant = LightAccountVariant::PoolState {
            seeds: PoolStateSeeds {
                mint_a: pool_state.token_a_mint,
                mint_b: pool_state.token_b_mint,
            },
            data: pool_state.clone(),
        };
        let spec = PdaSpec::new(interface, variant, PROGRAM_ID);
        self.pda_specs.insert(pool_pubkey, spec);

        Ok(())
    }

    /// Parse token vault from AccountInterface and store as PdaSpec.
    fn parse_token_vault(
        &mut self,
        account: &AccountInterface,
        is_vault_a: bool,
    ) -> Result<(), SwapSdkError> {
        let pool_state = self
            .pool_state_pubkey
            .ok_or(SwapSdkError::PoolStateNotParsed)?;

        let mint = if is_vault_a {
            self.token_a_mint
                .ok_or(SwapSdkError::MissingField("token_a_mint"))?
        } else {
            self.token_b_mint
                .ok_or(SwapSdkError::MissingField("token_b_mint"))?
        };

        let seeds = VaultSeeds {
            pool: pool_state.to_bytes(),
            mint: mint.to_bytes(),
        };

        // Parse token data from account
        let token_data = Token::deserialize(&mut account.data())
            .map_err(|e: std::io::Error| SwapSdkError::ParseError(e.to_string()))?;

        let variant = LightAccountVariant::Vault(light_account::token::TokenDataWithSeeds {
            seeds,
            token_data,
        });

        // For token vaults, convert ColdContext::Token to ColdContext::Account
        // because they're decompressed as PDAs, not as token accounts
        // (matches cp-swap pattern)
        let interface = if account.is_cold() {
            let compressed_account = match &account.cold {
                Some(ColdContext::Token(ct)) => ct.account.clone(),
                Some(ColdContext::Account(ca)) => ca.clone(),
                None => return Err(SwapSdkError::MissingField("cold_context")),
            };
            AccountInterface {
                key: account.key,
                account: account.account.clone(),
                cold: Some(ColdContext::Account(compressed_account)),
            }
        } else {
            account.clone()
        };

        let spec = PdaSpec::new(interface, variant, PROGRAM_ID);
        self.pda_specs.insert(account.key, spec);

        Ok(())
    }

    /// Parse mint from AccountInterface.
    fn parse_mint(&mut self, account: &AccountInterface) -> Result<(), SwapSdkError> {
        self.mint_specs.insert(account.key, account.clone());
        Ok(())
    }

    /// Parse any account and route to appropriate parser.
    fn parse_account(&mut self, account: &AccountInterface) -> Result<(), SwapSdkError> {
        // Check if this is a known vault by pubkey
        if Some(account.key) == self.token_a_vault {
            return self.parse_token_vault(account, true);
        }
        if Some(account.key) == self.token_b_vault {
            return self.parse_token_vault(account, false);
        }

        // Check discriminator for pool state
        let data = account.data();
        if data.len() >= 8 {
            let discriminator: [u8; 8] = data[..8].try_into().unwrap_or_default();

            if discriminator == PoolState::LIGHT_DISCRIMINATOR {
                return self.parse_pool_state(account.clone());
            }
        }

        // Check if this is a mint (token_a_mint or token_b_mint)
        if Some(account.key) == self.token_a_mint || Some(account.key) == self.token_b_mint {
            return self.parse_mint(account);
        }

        Ok(())
    }

    /// Check if pool state is cold.
    pub fn is_pool_state_cold(&self) -> bool {
        self.pool_state_pubkey
            .and_then(|k| self.pda_specs.get(&k))
            .is_some_and(|s| s.is_cold())
    }

    /// Check if token A vault is cold.
    pub fn is_vault_a_cold(&self) -> bool {
        self.token_a_vault
            .and_then(|k| self.pda_specs.get(&k))
            .is_some_and(|s| s.is_cold())
    }

    /// Check if token B vault is cold.
    pub fn is_vault_b_cold(&self) -> bool {
        self.token_b_vault
            .and_then(|k| self.pda_specs.get(&k))
            .is_some_and(|s| s.is_cold())
    }

    /// Get pool state pubkey.
    pub fn pool_state(&self) -> Option<Pubkey> {
        self.pool_state_pubkey
    }
}

impl LightProgramInterface for SwapSdk {
    type Variant = LightAccountVariant;
    type Instruction = SwapInstruction;
    type Error = SwapSdkError;

    fn program_id(&self) -> Pubkey {
        PROGRAM_ID
    }

    fn from_keyed_accounts(accounts: &[AccountInterface]) -> Result<Self, Self::Error> {
        let mut sdk = Self::new();

        // First pass: find and parse pool state
        for account in accounts {
            let data = account.data();
            if data.len() >= 8 {
                let discriminator: [u8; 8] = data[..8].try_into().unwrap_or_default();
                if discriminator == PoolState::LIGHT_DISCRIMINATOR {
                    sdk.parse_pool_state(account.clone())?;
                    break;
                }
            }
        }

        if sdk.pool_state_pubkey.is_none() {
            return Err(SwapSdkError::MissingField("pool_state"));
        }

        Ok(sdk)
    }

    fn get_accounts_to_update(&self, _ix: &Self::Instruction) -> Vec<AccountToFetch> {
        let mut accounts = Vec::new();

        // All instructions need pool_state
        if let Some(pubkey) = self.pool_state_pubkey {
            accounts.push(AccountToFetch::pda(pubkey, PROGRAM_ID));
        }

        // All instructions need token vaults
        if let Some(pubkey) = self.token_a_vault {
            accounts.push(AccountToFetch::token(pubkey));
        }
        if let Some(pubkey) = self.token_b_vault {
            accounts.push(AccountToFetch::token(pubkey));
        }

        // All instructions need mints
        if let Some(pubkey) = self.token_a_mint {
            accounts.push(AccountToFetch::mint(pubkey));
        }
        if let Some(pubkey) = self.token_b_mint {
            accounts.push(AccountToFetch::mint(pubkey));
        }

        accounts
    }

    fn update(&mut self, accounts: &[AccountInterface]) -> Result<(), Self::Error> {
        for account in accounts {
            self.parse_account(account)?;
        }
        Ok(())
    }

    fn get_all_specs(&self) -> Vec<AccountSpec<Self::Variant>> {
        let mut specs = Vec::new();

        // Add PDA specs (includes pool_state and vaults)
        for spec in self.pda_specs.values() {
            specs.push(AccountSpec::Pda(spec.clone()));
        }

        // Add mint specs
        for spec in self.mint_specs.values() {
            specs.push(AccountSpec::Mint(spec.clone()));
        }

        specs
    }

    fn get_specs_for_instruction(
        &self,
        _ix: &Self::Instruction,
    ) -> Vec<AccountSpec<Self::Variant>> {
        let mut specs = Vec::new();

        // Pool state needed for all instructions
        if let Some(pubkey) = self.pool_state_pubkey {
            if let Some(spec) = self.pda_specs.get(&pubkey) {
                specs.push(AccountSpec::Pda(spec.clone()));
            }
        }

        // Token vaults needed for all instructions
        if let Some(pubkey) = self.token_a_vault {
            if let Some(spec) = self.pda_specs.get(&pubkey) {
                specs.push(AccountSpec::Pda(spec.clone()));
            }
        }
        if let Some(pubkey) = self.token_b_vault {
            if let Some(spec) = self.pda_specs.get(&pubkey) {
                specs.push(AccountSpec::Pda(spec.clone()));
            }
        }

        // Mints needed for all instructions
        if let Some(pubkey) = self.token_a_mint {
            if let Some(spec) = self.mint_specs.get(&pubkey) {
                specs.push(AccountSpec::Mint(spec.clone()));
            }
        }
        if let Some(pubkey) = self.token_b_mint {
            if let Some(spec) = self.mint_specs.get(&pubkey) {
                specs.push(AccountSpec::Mint(spec.clone()));
            }
        }

        specs
    }
}
