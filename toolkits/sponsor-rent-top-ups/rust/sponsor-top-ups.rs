mod setup;

use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::{
    get_associated_token_address, CreateAssociatedTokenAccount,
    TransferInterface, LIGHT_TOKEN_PROGRAM_ID,
};
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(true, None)).await?;

    // Top-Up Sponsor: your application server, pays SOL for rent top-ups
    let sponsor = rpc.get_payer().insecure_clone();

    // User: only signs to authorize the transfer
    let sender = Keypair::new();

    let setup::SetupResult { mint, sender_ata } =
        setup::setup(&mut rpc, &sponsor, &sender).await?;

    // Create recipient associated token account
    let recipient = Keypair::new();
    let recipient_ata = get_associated_token_address(&recipient.pubkey(), &mint);
    let create_ata_ix =
        CreateAssociatedTokenAccount::new(sponsor.pubkey(), recipient.pubkey(), mint).instruction()?;
    rpc.create_and_send_transaction(&[create_ata_ix], &sponsor.pubkey(), &[&sponsor])
        .await?;

    let transfer_ix = TransferInterface {
        source: sender_ata,
        destination: recipient_ata,
        amount: 500_000,
        decimals: 6,
        authority: sender.pubkey(),
        payer: sponsor.pubkey(),
        spl_interface: None,
        source_owner: LIGHT_TOKEN_PROGRAM_ID,
        destination_owner: LIGHT_TOKEN_PROGRAM_ID,
    }
    .instruction()?;

    let sig = rpc
        .create_and_send_transaction(&[transfer_ix], &sponsor.pubkey(), &[&sponsor, &sender])
        .await?;

    println!("Tx: {sig}");

    Ok(())
}
