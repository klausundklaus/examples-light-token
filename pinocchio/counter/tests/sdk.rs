#![allow(dead_code)]

//! CounterSdk implementing LightProgramInterface trait.
//!
//! Provides:
//! - Parsing counter accounts from AccountInterface
//! - Tracking account state (hot/cold)
//! - Building AccountSpec for load instructions

use borsh::BorshDeserialize;
use light_account::LightDiscriminator;
use light_client::interface::{
    AccountInterface, AccountSpec, AccountToFetch, LightProgramInterface, PdaSpec,
};
use pinocchio_counter::{CounterState, CounterStateSeeds, LightAccountVariant};
use solana_pubkey::Pubkey;
use std::collections::HashMap;

pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(pinocchio_counter::ID);

/// Instructions supported by the counter program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterInstruction {
    CreateCounter,
    Increment,
}

/// Error type for SDK operations.
#[derive(Debug, Clone)]
pub enum CounterSdkError {
    ParseError(String),
    UnknownDiscriminator([u8; 8]),
    MissingField(&'static str),
    CounterStateNotParsed,
}

impl std::fmt::Display for CounterSdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::UnknownDiscriminator(disc) => write!(f, "Unknown discriminator: {:?}", disc),
            Self::MissingField(field) => write!(f, "Missing field: {}", field),
            Self::CounterStateNotParsed => write!(f, "Counter state must be parsed first"),
        }
    }
}

impl std::error::Error for CounterSdkError {}

/// SDK for managing counter accounts and building decompression instructions.
#[derive(Debug, Clone)]
pub struct CounterSdk {
    /// Counter state pubkey.
    pub counter_pubkey: Option<Pubkey>,
    /// Counter owner pubkey.
    pub owner: Option<Pubkey>,
    /// Cached PDA specs keyed by pubkey.
    pda_specs: HashMap<Pubkey, PdaSpec<LightAccountVariant>>,
}

impl Default for CounterSdk {
    fn default() -> Self {
        Self::new()
    }
}

impl CounterSdk {
    /// Create a new empty SDK instance.
    pub fn new() -> Self {
        Self {
            counter_pubkey: None,
            owner: None,
            pda_specs: HashMap::new(),
        }
    }

    /// Parse counter state from AccountInterface and populate SDK fields.
    fn parse_counter_state(
        &mut self,
        interface: AccountInterface,
    ) -> Result<(), CounterSdkError> {
        let data = interface.data();
        if data.len() < 8 {
            return Err(CounterSdkError::ParseError(
                "Account data too short".to_string(),
            ));
        }

        // Skip 8-byte discriminator
        let counter_state = CounterState::deserialize(&mut &data[8..])
            .map_err(|e| CounterSdkError::ParseError(e.to_string()))?;

        let counter_pubkey = interface.key;
        self.counter_pubkey = Some(counter_pubkey);
        self.owner = Some(Pubkey::new_from_array(counter_state.owner));

        // Create PdaSpec with variant
        let owner_bytes = counter_state.owner;
        let variant = LightAccountVariant::CounterState {
            seeds: CounterStateSeeds {
                owner: owner_bytes,
            },
            data: counter_state,
        };
        let spec = PdaSpec::new(interface, variant, PROGRAM_ID);
        self.pda_specs.insert(counter_pubkey, spec);

        Ok(())
    }
}

impl LightProgramInterface for CounterSdk {
    type Variant = LightAccountVariant;
    type Instruction = CounterInstruction;
    type Error = CounterSdkError;

    fn program_id(&self) -> Pubkey {
        PROGRAM_ID
    }

    fn from_keyed_accounts(accounts: &[AccountInterface]) -> Result<Self, Self::Error> {
        let mut sdk = Self::new();

        // Find and parse counter state
        for account in accounts {
            let data = account.data();
            if data.len() >= 8 {
                let discriminator: [u8; 8] = data[..8].try_into().unwrap_or_default();
                if discriminator == CounterState::LIGHT_DISCRIMINATOR {
                    sdk.parse_counter_state(account.clone())?;
                    break;
                }
            }
        }

        if sdk.counter_pubkey.is_none() {
            return Err(CounterSdkError::MissingField("counter_state"));
        }

        Ok(sdk)
    }

    fn get_accounts_to_update(&self, _ix: &Self::Instruction) -> Vec<AccountToFetch> {
        let mut accounts = Vec::new();

        if let Some(pubkey) = self.counter_pubkey {
            accounts.push(AccountToFetch::pda(pubkey, PROGRAM_ID));
        }

        accounts
    }

    fn update(&mut self, accounts: &[AccountInterface]) -> Result<(), Self::Error> {
        for account in accounts {
            let data = account.data();
            if data.len() >= 8 {
                let discriminator: [u8; 8] = data[..8].try_into().unwrap_or_default();
                if discriminator == CounterState::LIGHT_DISCRIMINATOR {
                    self.parse_counter_state(account.clone())?;
                }
            }
        }
        Ok(())
    }

    fn get_all_specs(&self) -> Vec<AccountSpec<Self::Variant>> {
        self.pda_specs
            .values()
            .map(|spec| AccountSpec::Pda(spec.clone()))
            .collect()
    }

    fn get_specs_for_instruction(
        &self,
        _ix: &Self::Instruction,
    ) -> Vec<AccountSpec<Self::Variant>> {
        let mut specs = Vec::new();

        if let Some(pubkey) = self.counter_pubkey {
            if let Some(spec) = self.pda_specs.get(&pubkey) {
                specs.push(AccountSpec::Pda(spec.clone()));
            }
        }

        specs
    }
}
