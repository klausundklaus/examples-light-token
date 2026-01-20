use anchor_lang::prelude::*;
use light_token::instruction::TransferInterfaceCpi;

/// Transfer tokens between Light token accounts.
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
) -> Result<()> {
    let cpi = TransferInterfaceCpi::new(
        amount,
        decimals,
        from,
        to,
        authority,
        payer,
        light_token_cpi_authority,
        system_program,
    );

    if let Some(seeds) = signer_seeds {
        cpi.invoke_signed(&[seeds])
    } else {
        cpi.invoke()
    }
    .map_err(|e| anchor_lang::prelude::ProgramError::from(e).into())
}
