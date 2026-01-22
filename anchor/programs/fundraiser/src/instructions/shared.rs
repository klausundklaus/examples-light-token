use anchor_lang::prelude::*;
use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{TransferCheckedCpi, TransferInterfaceCpi};

/// Configuration for SPL interface (required for SPL<->Light transfers)
pub struct SplInterfaceConfig<'info> {
    pub mint: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub spl_interface_pda: AccountInfo<'info>,
    pub spl_interface_pda_bump: u8,
}

/// Check if an account is a Light token account (owned by Light token program)
fn is_light_account(account: &AccountInfo) -> bool {
    account.owner == &Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID)
}

/// Transfer tokens between token accounts.
/// For Light-to-Light transfers, uses TransferCheckedCpi with mint validation.
/// For SPL<->Light transfers, uses TransferInterfaceCpi with spl_interface config.
pub fn transfer_tokens<'info>(
    amount: u64,
    decimals: u8,
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    payer: AccountInfo<'info>,
    light_token_cpi_authority: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    signer_seeds: Option<&[&[u8]]>,
    spl_interface: Option<SplInterfaceConfig<'info>>,
) -> Result<()> {
    // Check if this is a Light-to-Light transfer
    let is_light_to_light = is_light_account(&from) && is_light_account(&to);

    if is_light_to_light {
        // Use TransferCheckedCpi for Light-to-Light transfers
        // fee_payer: Some ensures authority is readonly (required for PDA with account data)
        let cpi = TransferCheckedCpi {
            source: from,
            mint,
            destination: to,
            amount,
            decimals,
            authority,
            system_program,
            max_top_up: Some(0), // Allow top-ups but with 0 limit
            fee_payer: Some(payer), // Payer handles any rent, makes authority readonly
        };

        if let Some(seeds) = signer_seeds {
            cpi.invoke_signed(&[seeds])
        } else {
            cpi.invoke()
        }
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e).into())
    } else {
        // Use TransferInterfaceCpi for SPL<->Light transfers
        let mut cpi = TransferInterfaceCpi::new(
            amount,
            decimals,
            from,
            to,
            authority,
            payer,
            light_token_cpi_authority,
            system_program,
        );

        // Add SPL interface if provided (required for SPL<->Light transfers)
        if let Some(spl) = spl_interface {
            cpi = cpi
                .with_spl_interface(
                    Some(spl.mint),
                    Some(spl.spl_token_program),
                    Some(spl.spl_interface_pda),
                    Some(spl.spl_interface_pda_bump),
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        }

        if let Some(seeds) = signer_seeds {
            cpi.invoke_signed(&[seeds])
        } else {
            cpi.invoke()
        }
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e).into())
    }
}
