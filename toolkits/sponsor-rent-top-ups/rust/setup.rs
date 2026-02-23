// Test setup: creates an SPL mint, funds the sponsor, wraps into Light.
// In production the mint already exists (e.g. USDC) and the sender
// already holds Light tokens.

use anchor_spl::token::spl_token;
use light_client::rpc::Rpc;
use light_program_test::LightProgramTest;
use light_token::{
    instruction::{
        get_associated_token_address, CreateAssociatedTokenAccount, SplInterface,
        TransferInterface, LIGHT_TOKEN_PROGRAM_ID,
    },
    spl_interface::find_spl_interface_pda_with_index,
};
use rust_client::{setup_spl_associated_token_account, setup_spl_mint};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

pub struct SetupResult {
    pub mint: Pubkey,
    pub sender_ata: Pubkey,
}

pub async fn setup(
    rpc: &mut LightProgramTest,
    sponsor: &Keypair,
    sender: &Keypair,
) -> Result<SetupResult, Box<dyn std::error::Error>> {
    let decimals = 6u8;
    let amount = 1_000_000u64;

    let mint = setup_spl_mint(rpc, sponsor, decimals).await;
    let sponsor_spl_ata = setup_spl_associated_token_account(
        rpc, sponsor, &mint, &sponsor.pubkey(), amount,
    ).await;
    let (interface_pda, interface_bump) = find_spl_interface_pda_with_index(&mint, 0, false);

    let sender_ata = get_associated_token_address(&sender.pubkey(), &mint);
    let create_ata_ix =
        CreateAssociatedTokenAccount::new(sponsor.pubkey(), sender.pubkey(), mint).instruction()?;
    rpc.create_and_send_transaction(&[create_ata_ix], &sponsor.pubkey(), &[sponsor])
        .await?;

    let spl_interface = SplInterface {
        mint,
        spl_token_program: spl_token::ID,
        spl_interface_pda: interface_pda,
        spl_interface_pda_bump: interface_bump,
    };

    let wrap_ix = TransferInterface {
        source: sponsor_spl_ata,
        destination: sender_ata,
        amount,
        decimals,
        authority: sponsor.pubkey(),
        payer: sponsor.pubkey(),
        spl_interface: Some(spl_interface),
        source_owner: spl_token::ID,
        destination_owner: LIGHT_TOKEN_PROGRAM_ID,
    }
    .instruction()?;

    rpc.create_and_send_transaction(&[wrap_ix], &sponsor.pubkey(), &[sponsor])
        .await?;

    Ok(SetupResult { mint, sender_ata })
}
