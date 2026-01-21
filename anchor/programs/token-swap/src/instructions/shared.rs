use anchor_lang::prelude::*;
use light_token::instruction::TransferInterfaceCpi;

/// Configuration for SPL interface (required for SPL<->Light transfers)
pub struct SplInterfaceConfig<'info> {
    pub mint: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub spl_interface_pda: AccountInfo<'info>,
    pub spl_interface_pda_bump: u8,
}

/// Transfer tokens between Light token accounts.
/// For SPL<->Light transfers, pass spl_interface config.
pub fn transfer_tokens<'info>(
    amount: u64,
    decimals: u8,
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    payer: AccountInfo<'info>,
    light_token_cpi_authority: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    signer_seeds: Option<&[&[u8]]>,
    spl_interface: Option<SplInterfaceConfig<'info>>,
) -> Result<()> {
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
