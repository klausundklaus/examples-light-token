//! Initialize instruction processor.
//!
//! Creates 2 mints, 2 vaults, and initializes pool state.

use light_account_pinocchio::{
    prepare_compressed_account_on_init, CompressedCpiContext, CpiAccounts, CpiAccountsConfig,
    CpiContextWriteAccounts, CreateMints, CreateMintsStaticAccounts, CreateTokenAccountCpi,
    InstructionDataInvokeCpiWithAccountInfo, InvokeLightSystemProgram, LightAccount, LightConfig,
    LightSdkTypesError, PackedAddressTreeInfoExt, SingleMintParams,
};
use pinocchio::{
    account_info::AccountInfo,
    sysvars::{clock::Clock, Sysvar},
};

use super::accounts::{InitializeAccounts, InitializeParams};
use crate::constants::*;

/// Create a token vault PDA with rent-free storage.
fn create_vault(
    ctx: &InitializeAccounts<'_>,
    pool_key: &[u8; 32],
    mint: &AccountInfo,
    vault: &AccountInfo,
    bump: u8,
) -> Result<(), LightSdkTypesError> {
    let mint_key = mint.key();
    let seeds: &[&[u8]] = &[
        POOL_VAULT_SEED,
        pool_key.as_ref(),
        mint_key.as_ref(),
        &[bump],
    ];

    CreateTokenAccountCpi {
        payer: ctx.payer,
        account: vault,
        mint,
        owner: *ctx.pool_authority.key(),
    }
    .rent_free(
        ctx.light_token_config,
        ctx.light_token_rent_sponsor,
        ctx.system_program,
        &crate::ID,
    )
    .invoke_signed(seeds)
}

/// Process the initialize instruction.
pub fn process(
    ctx: &InitializeAccounts<'_>,
    params: &InitializeParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    const NUM_LIGHT_PDAS: usize = 1; // Pool state
    const NUM_LIGHT_MINTS: usize = 2; // Mint A and Mint B
    const WITH_CPI_CONTEXT: bool = true;

    // 1. Build CPI accounts
    let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
    if remaining_accounts.len() < system_accounts_offset {
        return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
    }
    let config = CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        ctx.payer,
        &remaining_accounts[system_accounts_offset..],
        config,
    );

    // 2. Address tree info
    let address_tree_info = &params.create_accounts_proof.address_tree_info;
    let address_tree_pubkey = address_tree_info
        .get_tree_pubkey(&cpi_accounts)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
    let output_tree_index = params.create_accounts_proof.output_state_tree_index;

    // 3. Load config, get slot
    let light_config = LightConfig::load_checked(ctx.compressible_config, &crate::ID)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
    let current_slot = Clock::get()
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?
        .slot;

    // 4. Create Pool PDA via invoke_write_to_cpi_context_first
    {
        let cpi_context = CompressedCpiContext::first();
        let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
        let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);

        let pool_key = *ctx.pool.key();
        prepare_compressed_account_on_init(
            &pool_key,
            &address_tree_pubkey,
            address_tree_info,
            output_tree_index,
            0,
            &crate::ID,
            &mut new_address_params,
            &mut account_infos,
        )?;

        // Initialize pool state data
        {
            let mut account_data = ctx
                .pool
                .try_borrow_mut_data()
                .map_err(|_| LightSdkTypesError::Borsh)?;

            // Find global authority bump (single authority for all pools)
            let mint_a_key = ctx.mint_a().key();
            let mint_b_key = ctx.mint_b().key();
            let (_, authority_bump) =
                pinocchio::pubkey::find_program_address(&[POOL_AUTHORITY_SEED], &crate::ID);

            // Zero-copy initialization
            let record_bytes =
                &mut account_data[8..8 + core::mem::size_of::<crate::state::PoolState>()];
            let pool_state: &mut crate::state::PoolState = bytemuck::from_bytes_mut(record_bytes);

            pool_state.set_decompressed(&light_config, current_slot);
            pool_state.authority_bump = authority_bump;
            pool_state.token_a_mint = *mint_a_key;
            pool_state.token_b_mint = *mint_b_key;
            pool_state.token_a_vault = *ctx.vault_a().key();
            pool_state.token_b_vault = *ctx.vault_b().key();
            pool_state.lp_supply = 0;
            pool_state.fee_bps = params.fee_bps;
            pool_state.admin = *ctx.authority.key();
        }

        // Write to CPI context
        let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
            mode: 1,
            bump: crate::LIGHT_CPI_SIGNER.bump,
            invoking_program_id: crate::LIGHT_CPI_SIGNER.program_id.into(),
            compress_or_decompress_lamports: 0,
            is_compress: false,
            with_cpi_context: WITH_CPI_CONTEXT,
            with_transaction_hash: false,
            cpi_context,
            proof: params.create_accounts_proof.proof.0,
            new_address_params,
            account_infos,
            read_only_addresses: vec![],
            read_only_accounts: vec![],
        };

        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority()?,
            cpi_context: cpi_accounts.cpi_context()?,
            cpi_signer: crate::LIGHT_CPI_SIGNER,
        };
        instruction_data.invoke_write_to_cpi_context_first(cpi_context_accounts)?;
    }

    // 5. Create Mints (A and B)
    {
        let authority_key = *ctx.authority.key();
        let mint_signer_a_key = *ctx.mint_signer_a().key();
        let mint_signer_b_key = *ctx.mint_signer_b().key();

        let mint_signer_a_seeds: &[&[u8]] = &[
            MINT_A_SEED,
            authority_key.as_ref(),
            &[params.mint_signer_a_bump],
        ];

        let mint_signer_b_seeds: &[&[u8]] = &[
            MINT_B_SEED,
            authority_key.as_ref(),
            &[params.mint_signer_b_bump],
        ];

        let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [
            SingleMintParams {
                decimals: MINT_DECIMALS,
                mint_authority: authority_key,
                mint_bump: None,
                freeze_authority: None,
                mint_seed_pubkey: mint_signer_a_key,
                authority_seeds: None,
                mint_signer_seeds: Some(mint_signer_a_seeds),
                token_metadata: None,
            },
            SingleMintParams {
                decimals: MINT_DECIMALS,
                mint_authority: authority_key,
                mint_bump: None,
                freeze_authority: None,
                mint_seed_pubkey: mint_signer_b_key,
                authority_seeds: None,
                mint_signer_seeds: Some(mint_signer_b_seeds),
                token_metadata: None,
            },
        ];

        CreateMints {
            mints: &sdk_mints,
            proof_data: &params.create_accounts_proof,
            mint_seed_accounts: ctx.mint_signers,
            mint_accounts: ctx.mints,
            static_accounts: CreateMintsStaticAccounts {
                fee_payer: ctx.payer,
                compressible_config: ctx.light_token_config,
                rent_sponsor: ctx.light_token_rent_sponsor,
                cpi_authority: ctx.cpi_authority,
            },
            cpi_context_offset: NUM_LIGHT_PDAS as u8,
        }
        .invoke(&cpi_accounts)?;
    }

    // 6. Create Vaults
    let pool_key = *ctx.pool.key();
    create_vault(
        ctx,
        &pool_key,
        ctx.mint_a(),
        ctx.vault_a(),
        params.vault_a_bump,
    )?;
    create_vault(
        ctx,
        &pool_key,
        ctx.mint_b(),
        ctx.vault_b(),
        params.vault_b_bump,
    )?;

    Ok(())
}
