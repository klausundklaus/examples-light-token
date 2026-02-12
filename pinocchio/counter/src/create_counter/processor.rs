//! Create counter instruction processor.
//!
//! Creates a counter PDA and registers it with Light Protocol for compression.

use light_account_pinocchio::{
    prepare_compressed_account_on_init, CpiAccounts, CpiAccountsConfig,
    InstructionDataInvokeCpiWithAccountInfo, InvokeLightSystemProgram, LightAccount, LightConfig,
    LightSdkTypesError, PackedAddressTreeInfoExt,
};
use pinocchio::{
    account_info::AccountInfo,
    sysvars::{clock::Clock, Sysvar},
};

use super::accounts::{CreateCounterAccounts, CreateCounterParams};

/// Process the create_counter instruction.
pub fn process(
    ctx: &CreateCounterAccounts<'_>,
    params: &CreateCounterParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    // 1. Build CPI accounts (no CPI context — single PDA, no batching)
    let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
    if remaining_accounts.len() < system_accounts_offset {
        return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
    }
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
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

    // 4. Prepare compressed account on init
    let mut new_address_params = Vec::with_capacity(1);
    let mut account_infos = Vec::with_capacity(1);

    let counter_key = *ctx.counter.key();
    prepare_compressed_account_on_init(
        &counter_key,
        &address_tree_pubkey,
        address_tree_info,
        output_tree_index,
        0,
        &crate::ID,
        &mut new_address_params,
        &mut account_infos,
    )?;

    // 5. Initialize counter state data via zero-copy
    {
        let mut account_data = ctx
            .counter
            .try_borrow_mut_data()
            .map_err(|_| LightSdkTypesError::Borsh)?;

        let record_bytes =
            &mut account_data[8..8 + core::mem::size_of::<crate::state::CounterState>()];
        let counter_state: &mut crate::state::CounterState =
            bytemuck::from_bytes_mut(record_bytes);

        counter_state.set_decompressed(&light_config, current_slot);
        counter_state.owner = *ctx.owner.key();
        counter_state.count = 0;
    }

    // 6. Invoke Light system program CPI
    let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
        mode: 1,
        bump: crate::LIGHT_CPI_SIGNER.bump,
        invoking_program_id: crate::LIGHT_CPI_SIGNER.program_id.into(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: false,
        with_transaction_hash: false,
        cpi_context: Default::default(),
        proof: params.create_accounts_proof.proof.0,
        new_address_params,
        account_infos,
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    instruction_data.invoke(cpi_accounts)?;

    Ok(())
}
