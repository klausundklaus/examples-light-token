//! Shared test utilities for Light Protocol example programs.
//!
//! This crate provides helper functions for creating and managing:
//! - SPL mints and token accounts
//! - Token-2022 mints and token accounts
//! - Light Protocol token accounts
//! - SPL interface PDAs for SPL<->Light transfers
//! - Balance verification utilities

use anchor_spl::token;
use light_token::spl_interface::{find_spl_interface_pda, CreateSplInterfacePda};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_token_2022::pod::PodAccount;

// Re-exports for convenience - these are what consumers of this crate will use
pub use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, CreateAccountsProofResult,
    InitializeRentFreeConfig,
};
pub use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest, TestRpc},
    Indexer, ProgramTestConfig, Rpc,
};
pub use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
pub use light_token::constants::CPI_AUTHORITY_PDA;
pub use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

// Re-export light-token-minter program ID for tests that need to create Light mints
pub use light_token_minter::ID as LIGHT_TOKEN_MINTER_PROGRAM_ID;

/// Token type for mint creation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MintType {
    /// Standard SPL Token (token::ID)
    Spl,
    /// Token-2022 (spl_token_2022::ID)
    Token2022,
}

impl MintType {
    pub fn program_id(&self) -> Pubkey {
        match self {
            MintType::Spl => token::ID,
            MintType::Token2022 => spl_token_2022::ID,
        }
    }
}

/// Result of creating an SPL interface PDA
pub struct SplInterfaceResult {
    pub pda: Pubkey,
    pub bump: u8,
}

// ============================================================================
// Setup Module
// ============================================================================

pub mod setup {
    use super::*;

    /// Initialize rent-free config for a program
    pub async fn initialize_rent_free_config<R: Rpc + TestRpc>(
        rpc: &mut R,
        payer: &Keypair,
        program_id: &Pubkey,
        rent_sponsor: Pubkey,
    ) -> Pubkey {
        // Setup program data for rent-free config
        let program_data_pda = setup_mock_program_data(rpc, payer, program_id);

        // Initialize rent-free config
        let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
            program_id,
            &payer.pubkey(),
            &program_data_pda,
            rent_sponsor,
            payer.pubkey(),
        )
        .build();

        rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[payer])
            .await
            .expect("Initialize rent-free config should succeed");

        println!("Rent-free config initialized at: {:?}", config_pda);
        config_pda
    }

    /// Create a new LightProgramTest instance with the given program
    pub async fn create_program_test(
        program_name: &'static str,
        program_id: Pubkey,
    ) -> (LightProgramTest, Keypair) {
        let mut config = ProgramTestConfig::new_v2(true, Some(vec![(program_name, program_id)]));
        config = config.with_light_protocol_events();

        let rpc = LightProgramTest::new(config).await.unwrap();
        let payer = rpc.get_payer().insecure_clone();
        (rpc, payer)
    }
}

// ============================================================================
// SPL Tokens Module
// ============================================================================

pub mod spl_tokens {
    use super::*;

    /// Create an SPL mint with the given decimals
    pub async fn create_spl_mint<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint_authority: &Pubkey,
        decimals: u8,
    ) -> Keypair {
        create_mint_internal(rpc, payer, mint_authority, decimals, MintType::Spl).await
    }

    /// Create an SPL ATA for the given owner and mint
    pub async fn create_spl_ata<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Pubkey {
        create_ata_internal(rpc, payer, mint, owner, MintType::Spl).await
    }

    /// Mint SPL tokens to an account
    pub async fn mint_spl_tokens<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint: &Pubkey,
        destination: &Pubkey,
        mint_authority: &Keypair,
        amount: u64,
    ) {
        mint_tokens_internal(
            rpc,
            payer,
            mint,
            destination,
            mint_authority,
            amount,
            MintType::Spl,
        )
        .await
    }
}

// ============================================================================
// Token-2022 Module
// ============================================================================

pub mod t22_tokens {
    use super::*;

    /// Create a Token-2022 mint with the given decimals (no extensions)
    pub async fn create_t22_mint<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint_authority: &Pubkey,
        decimals: u8,
    ) -> Keypair {
        create_mint_internal(rpc, payer, mint_authority, decimals, MintType::Token2022).await
    }

    /// Create a Token-2022 ATA for the given owner and mint
    pub async fn create_t22_ata<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Pubkey {
        create_ata_internal(rpc, payer, mint, owner, MintType::Token2022).await
    }

    /// Mint Token-2022 tokens to an account
    pub async fn mint_t22_tokens<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint: &Pubkey,
        destination: &Pubkey,
        mint_authority: &Keypair,
        amount: u64,
    ) {
        mint_tokens_internal(
            rpc,
            payer,
            mint,
            destination,
            mint_authority,
            amount,
            MintType::Token2022,
        )
        .await
    }
}

// ============================================================================
// Light Tokens Module
// ============================================================================

pub mod light_tokens {
    use super::*;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use light_token::instruction::{
        derive_associated_token_account, derive_token_ata, find_mint_address, CompressibleParams,
        CreateAssociatedTokenAccount, TokenDataVersion,
    };
    use solana_instruction::Instruction;

    /// Result of creating a Light mint
    pub struct LightMintResult {
        /// The mint PDA
        pub mint: Pubkey,
        /// The mint signer PDA (derived from authority)
        pub mint_signer: Pubkey,
        /// The bump for the mint signer PDA
        pub mint_signer_bump: u8,
        /// The authority keypair (used as mint authority)
        pub authority: Keypair,
    }

    /// Create a Light mint using the light-token-minter program.
    ///
    /// This creates a new Light Protocol compressed mint with metadata.
    /// Returns information about the created mint including the mint PDA,
    /// mint signer PDA, and authority keypair.
    ///
    /// # Arguments
    /// * `rpc` - RPC client (must implement Rpc + Indexer)
    /// * `payer` - Transaction fee payer
    /// * `decimals` - Token decimals
    /// * `token_name` - Token name for metadata
    /// * `token_symbol` - Token symbol for metadata
    /// * `compression_config` - The compression config PDA (from initialize_rent_free_config)
    ///
    /// # Returns
    /// * `LightMintResult` - Contains mint PDA, mint_signer PDA, bump, and authority keypair
    pub async fn create_light_mint<R: Rpc + Indexer>(
        rpc: &mut R,
        payer: &Keypair,
        decimals: u8,
        token_name: &str,
        token_symbol: &str,
        compression_config: &Pubkey,
    ) -> LightMintResult {
        let program_id = light_token_minter::ID;

        // Create a new authority keypair for this mint
        let authority = Keypair::new();

        // Derive mint signer PDA from authority
        let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
            &[
                light_token_minter::MINT_SIGNER_SEED,
                authority.pubkey().as_ref(),
            ],
            &program_id,
        );

        // Derive mint PDA from mint signer
        let (mint_pda, _) = find_mint_address(&mint_signer_pda);

        // Get proof for creating the mint
        let proof_result = get_create_accounts_proof(
            rpc,
            &program_id,
            vec![CreateAccountsProofInput::mint(mint_signer_pda)],
        )
        .await
        .expect("Get create accounts proof should succeed");

        // Build create_mint instruction
        let accounts = light_token_minter::accounts::CreateMint {
            fee_payer: payer.pubkey(),
            authority: authority.pubkey(),
            mint_signer: mint_signer_pda,
            cmint: mint_pda,
            compression_config: *compression_config,
            light_token_config: LIGHT_TOKEN_CONFIG,
            light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
            light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            light_token_cpi_authority: CPI_AUTHORITY_PDA,
            system_program: solana_sdk::system_program::ID,
        };

        let instruction_data = light_token_minter::instruction::CreateMint {
            params: light_token_minter::CreateMintParams {
                create_accounts_proof: proof_result.create_accounts_proof,
                decimals,
                mint_signer_bump,
                token_name: token_name.to_string(),
                token_symbol: token_symbol.to_string(),
                token_uri: format!("https://example.com/{}.json", token_symbol.to_lowercase()),
            },
        };

        let instruction = Instruction {
            program_id,
            accounts: [
                accounts.to_account_metas(None),
                proof_result.remaining_accounts,
            ]
            .concat(),
            data: instruction_data.data(),
        };

        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &authority])
            .await
            .expect("Create Light mint should succeed");

        println!("Light mint created: {:?}", mint_pda);
        println!("  Mint signer: {:?}", mint_signer_pda);
        println!("  Authority: {:?}", authority.pubkey());
        println!("  Decimals: {}", decimals);

        LightMintResult {
            mint: mint_pda,
            mint_signer: mint_signer_pda,
            mint_signer_bump,
            authority,
        }
    }

    /// Mint Light tokens to a Light ATA, creating the ATA if needed.
    ///
    /// This mints tokens from a Light mint to a destination owner's Light ATA.
    /// The ATA is created automatically if it doesn't exist.
    ///
    /// # Arguments
    /// * `rpc` - RPC client (must implement Rpc + Indexer)
    /// * `payer` - Transaction fee payer
    /// * `mint_authority` - Keypair that is the mint authority
    /// * `mint` - The Light mint PDA
    /// * `destination_owner` - Owner of the destination ATA
    /// * `amount` - Amount of tokens to mint
    ///
    /// # Returns
    /// * `Pubkey` - The destination ATA address
    pub async fn mint_light_tokens<R: Rpc + Indexer>(
        rpc: &mut R,
        payer: &Keypair,
        mint_authority: &Keypair,
        mint: &Pubkey,
        destination_owner: &Pubkey,
        amount: u64,
    ) -> Pubkey {
        let program_id = light_token_minter::ID;

        // Derive the ATA address
        let (destination_ata, ata_bump) = derive_token_ata(destination_owner, mint);

        // Get proof for creating the ATA (empty vec - ATA creation via macro)
        let proof_result = get_create_accounts_proof(rpc, &program_id, vec![])
            .await
            .expect("Get create accounts proof should succeed");

        // Build mint_to instruction
        let accounts = light_token_minter::accounts::MintTo {
            fee_payer: payer.pubkey(),
            mint_authority: mint_authority.pubkey(),
            mint: *mint,
            recipient: *destination_owner,
            destination: destination_ata,
            light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            system_program: solana_sdk::system_program::ID,
            light_token_config: LIGHT_TOKEN_CONFIG,
            light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
            light_token_cpi_authority: CPI_AUTHORITY_PDA,
        };

        let instruction_data = light_token_minter::instruction::MintTo {
            params: light_token_minter::MintTokenParams {
                amount,
                create_accounts_proof: proof_result.create_accounts_proof,
                ata_bump,
            },
        };

        let instruction = Instruction {
            program_id,
            accounts: [
                accounts.to_account_metas(None),
                proof_result.remaining_accounts,
            ]
            .concat(),
            data: instruction_data.data(),
        };

        // Sign with both payer and mint_authority
        let signers: Vec<&Keypair> = if payer.pubkey() == mint_authority.pubkey() {
            vec![payer]
        } else {
            vec![payer, mint_authority]
        };

        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
            .await
            .expect("Mint Light tokens should succeed");

        println!(
            "Minted {} Light tokens to {:?} (owner: {:?})",
            amount, destination_ata, destination_owner
        );

        destination_ata
    }

    /// Create a Light token account (ATA) for the given owner and mint
    ///
    /// This creates a compressible Light Protocol token account that can hold
    /// compressed tokens.
    pub async fn create_light_ata<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Pubkey {
        let compressible_params = CompressibleParams {
            compressible_config: LIGHT_TOKEN_CONFIG,
            rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(1000),
            compress_to_account_pubkey: None,
            token_account_version: TokenDataVersion::ShaFlat,
            compression_only: true,
        };

        let create_ata_ix = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, *mint)
            .with_compressible(compressible_params)
            .idempotent()
            .instruction()
            .expect("Create Light ATA instruction should succeed");

        rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[payer])
            .await
            .expect("Create Light ATA should succeed");

        let (ata, _) = derive_associated_token_account(owner, mint);
        println!("Light ATA created: {:?} for owner {:?}", ata, owner);
        ata
    }

    /// Derive the Light token account address for an owner and mint
    pub fn derive_light_ata(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
        derive_associated_token_account(owner, mint)
    }

    /// Derive the Light token ATA address using light_token::derive_token_ata
    pub fn derive_light_token_ata(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
        derive_token_ata(owner, mint)
    }
}

// ============================================================================
// SPL Interface Module
// ============================================================================

pub mod spl_interface {
    use super::*;
    use light_token::instruction::TransferFromSpl;

    /// Create an SPL interface PDA for the given mint
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer
    /// * `mint` - The mint public key
    /// * `mint_type` - Whether this is SPL or Token-2022
    /// * `restricted` - Whether the mint has restricted T22 extensions
    pub async fn create_spl_interface_pda<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        mint: &Pubkey,
        mint_type: MintType,
        restricted: bool,
    ) -> SplInterfaceResult {
        let (pda, bump) = find_spl_interface_pda(mint, restricted);

        let create_ix =
            CreateSplInterfacePda::new(payer.pubkey(), *mint, mint_type.program_id(), restricted)
                .instruction();

        rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[payer])
            .await
            .expect("Create SPL interface PDA should succeed");

        println!(
            "SPL interface PDA created: {:?} (restricted={})",
            pda, restricted
        );
        SplInterfaceResult { pda, bump }
    }

    /// Get the SPL interface PDA address without creating it
    pub fn get_spl_interface_pda(mint: &Pubkey, restricted: bool) -> SplInterfaceResult {
        let (pda, bump) = find_spl_interface_pda(mint, restricted);
        SplInterfaceResult { pda, bump }
    }

    /// Transfer tokens from an SPL/T22 token account to a Light token account (compress)
    ///
    /// This transfers SPL tokens from a standard SPL/T22 ATA to a Light Protocol
    /// compressed token account, effectively "compressing" the tokens.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer
    /// * `authority` - Authority for the source SPL account (must be signer)
    /// * `mint` - The mint public key
    /// * `decimals` - Token decimals
    /// * `source_spl_ata` - Source SPL/T22 token account
    /// * `destination_light_ata` - Destination Light token account
    /// * `spl_interface_pda` - SPL interface PDA (created via create_spl_interface_pda)
    /// * `spl_interface_bump` - Bump seed for the SPL interface PDA
    /// * `amount` - Amount of tokens to transfer
    /// * `mint_type` - Whether this is SPL or Token-2022
    pub async fn transfer_spl_to_light<R: Rpc>(
        rpc: &mut R,
        payer: &Keypair,
        authority: &Keypair,
        mint: &Pubkey,
        decimals: u8,
        source_spl_ata: &Pubkey,
        destination_light_ata: &Pubkey,
        spl_interface_pda: &Pubkey,
        spl_interface_bump: u8,
        amount: u64,
        mint_type: MintType,
    ) {
        let transfer_ix = TransferFromSpl {
            amount,
            spl_interface_pda_bump: spl_interface_bump,
            decimals,
            source_spl_token_account: *source_spl_ata,
            destination: *destination_light_ata,
            authority: authority.pubkey(),
            mint: *mint,
            payer: payer.pubkey(),
            spl_interface_pda: *spl_interface_pda,
            spl_token_program: mint_type.program_id(),
        }
        .instruction()
        .expect("TransferFromSpl instruction should succeed");

        // Sign with both payer and authority if they're different
        let signers: Vec<&Keypair> = if payer.pubkey() == authority.pubkey() {
            vec![payer]
        } else {
            vec![payer, authority]
        };

        rpc.create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &signers)
            .await
            .expect("Transfer SPL to Light should succeed");

        println!(
            "Transferred {} tokens from SPL {:?} to Light {:?}",
            amount, source_spl_ata, destination_light_ata
        );
    }
}

// ============================================================================
// Helpers Module
// ============================================================================

pub mod helpers {
    use super::*;

    /// Verify Light Token account balance
    pub async fn verify_light_token_balance<R: Rpc>(
        rpc: &mut R,
        account: Pubkey,
        expected: u64,
        name: &str,
    ) {
        let account_data = rpc.get_account(account).await.unwrap();

        if let Some(data) = account_data {
            // Light Token accounts have first 165 bytes as SPL-compatible token account data
            if data.data.len() >= 165 {
                let token_state =
                    spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&data.data[..165]).unwrap();
                let actual = u64::from(token_state.amount);
                println!("{}: balance = {} (expected {})", name, actual, expected);
                assert_eq!(
                    actual, expected,
                    "{} balance mismatch: expected {}, got {}",
                    name, expected, actual
                );
            } else {
                panic!(
                    "{}: account data too short: {} bytes",
                    name,
                    data.data.len()
                );
            }
        } else if expected == 0 {
            println!("{}: account not found (expected 0 balance)", name);
        } else {
            panic!(
                "{}: account not found but expected balance {}",
                name, expected
            );
        }
    }

    /// Verify SPL/T22 token account balance
    pub async fn verify_spl_token_balance<R: Rpc>(
        rpc: &mut R,
        account: Pubkey,
        expected: u64,
        name: &str,
    ) {
        let account_data = rpc
            .get_account(account)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("{} should exist", name));

        let token_state =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&account_data.data[..165]).unwrap();
        let actual = u64::from(token_state.amount);
        println!("{}: balance = {} (expected {})", name, actual, expected);
        assert_eq!(
            actual, expected,
            "{} balance mismatch: expected {}, got {}",
            name, expected, actual
        );
    }

    /// Get proof for creating Light token accounts (PDAs)
    pub async fn get_creation_proof<R: Rpc + Indexer>(
        rpc: &R,
        program_id: &Pubkey,
        accounts: Vec<Pubkey>,
    ) -> CreateAccountsProofResult {
        let inputs: Vec<CreateAccountsProofInput> = accounts
            .into_iter()
            .map(CreateAccountsProofInput::pda)
            .collect();

        get_create_accounts_proof(rpc, program_id, inputs)
            .await
            .expect("Get creation proof should succeed")
    }

    /// Airdrop lamports to an account
    pub async fn airdrop<R: Rpc>(rpc: &mut R, pubkey: &Pubkey, lamports: u64) {
        rpc.airdrop_lamports(pubkey, lamports)
            .await
            .expect("Airdrop should succeed");
    }
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

/// SPL mint account size (fixed at 82 bytes)
const SPL_MINT_SIZE: usize = 82;

/// Token-2022 mint account size (base size without extensions)
const T22_MINT_SIZE: usize = 82;

async fn create_mint_internal<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
    mint_type: MintType,
) -> Keypair {
    let mint = Keypair::new();
    let program_id = mint_type.program_id();

    // Get the mint account size based on token program
    let mint_size = match mint_type {
        MintType::Spl => SPL_MINT_SIZE,
        MintType::Token2022 => T22_MINT_SIZE,
    };

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(mint_size)
        .await
        .unwrap();

    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        mint_size as u64,
        &program_id,
    );

    let init_mint_ix = match mint_type {
        MintType::Spl => token::spl_token::instruction::initialize_mint(
            &program_id,
            &mint.pubkey(),
            mint_authority,
            None,
            decimals,
        )
        .unwrap(),
        MintType::Token2022 => spl_token_2022::instruction::initialize_mint(
            &program_id,
            &mint.pubkey(),
            mint_authority,
            None,
            decimals,
        )
        .unwrap(),
    };

    rpc.create_and_send_transaction(
        &[create_account_ix, init_mint_ix],
        &payer.pubkey(),
        &[payer, &mint],
    )
    .await
    .expect("Create mint should succeed");

    println!("{:?} mint created: {:?}", mint_type, mint.pubkey());
    mint
}

async fn create_ata_internal<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    mint_type: MintType,
) -> Pubkey {
    let program_id = mint_type.program_id();

    let ata = anchor_spl::associated_token::spl_associated_token_account::get_associated_token_address_with_program_id(
        owner,
        mint,
        &program_id,
    );

    let create_ata_ix =
        anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            owner,
            mint,
            &program_id,
        );

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[payer])
        .await
        .expect("Create ATA should succeed");

    println!(
        "{:?} ATA created: {:?} for owner {:?}",
        mint_type, ata, owner
    );
    ata
}

async fn mint_tokens_internal<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
    mint_type: MintType,
) {
    let program_id = mint_type.program_id();

    let mint_to_ix = match mint_type {
        MintType::Spl => token::spl_token::instruction::mint_to(
            &program_id,
            mint,
            destination,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap(),
        MintType::Token2022 => spl_token_2022::instruction::mint_to(
            &program_id,
            mint,
            destination,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap(),
    };

    // If payer is the same as mint_authority, only sign once
    let signers: Vec<&Keypair> = if payer.pubkey() == mint_authority.pubkey() {
        vec![payer]
    } else {
        vec![payer, mint_authority]
    };

    rpc.create_and_send_transaction(&[mint_to_ix], &payer.pubkey(), &signers)
        .await
        .expect("Mint tokens should succeed");

    println!("Minted {} tokens to {:?}", amount, destination);
}

/// Common test constants
pub mod constants {
    /// Default decimals for test tokens
    pub const DEFAULT_DECIMALS: u8 = 9;

    /// Default token amount (1000 tokens with 9 decimals)
    pub const DEFAULT_TOKEN_AMOUNT: u64 = 1_000_000_000_000;

    /// Default airdrop amount (10 SOL)
    pub const DEFAULT_AIRDROP: u64 = 10_000_000_000;
}
