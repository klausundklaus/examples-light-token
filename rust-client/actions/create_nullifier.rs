//! Example: Create a nullifier PDA to prevent duplicate actions.
//!
//! This example uses LightProgramTest for local testing.
//! For devnet, replace LightProgramTest with LightClient (see comments below).
//!
//! Run with: cargo run --example action_create_nullifier

use light_client::rpc::Rpc;
use light_nullifier_program::sdk::{create_nullifier_ix, derive_nullifier_address, PROGRAM_ID};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_system_interface::instruction as system_instruction;

// For devnet usage with LightClient:
// ```rust
// use light_client::rpc::{LightClient, LightClientConfig};
// let api_key = std::env::var("API_KEY").expect("API_KEY required");
// let config = LightClientConfig::new(
//     format!("https://devnet.helius-rpc.com/?api-key={}", api_key),
//     None,
//     Some(api_key),
// );
// let mut rpc = LightClient::new(config).await?;
// ```

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Nullifier Program ID: {}", PROGRAM_ID);

    // Setup local test environment
    // For devnet: use LightClient instead (see comments above)
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(true, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    println!("Payer: {}", payer.pubkey());

    // Create a unique 32-byte ID (e.g., hash of payment inputs)
    let id: [u8; 32] = rand::random();
    println!("Nullifier ID: {:?}", &id[..8]);

    // Build nullifier instruction
    let nullifier_ix = create_nullifier_ix(&mut rpc, payer.pubkey(), id).await?;

    // Combine with a simple transfer (example of prepending nullifier)
    let recipient = Keypair::new().pubkey();
    let transfer_ix = system_instruction::transfer(&payer.pubkey(), &recipient, 1_000);

    // Build and send transaction
    rpc.create_and_send_transaction(&[nullifier_ix, transfer_ix], &payer.pubkey(), &[&payer])
        .await?;
    println!("Transaction confirmed");

    // Verify nullifier was created
    let address = derive_nullifier_address(&id);
    println!(
        "Nullifier address: {}",
        bs58::encode(&address).into_string()
    );

    // Try to create the same nullifier again (should fail)
    println!("\nAttempting duplicate nullifier (should fail)...");
    let nullifier_ix_2 = create_nullifier_ix(&mut rpc, payer.pubkey(), id).await?;
    let transfer_ix_2 = system_instruction::transfer(&payer.pubkey(), &recipient, 1_000);

    match rpc
        .create_and_send_transaction(&[nullifier_ix_2, transfer_ix_2], &payer.pubkey(), &[&payer])
        .await
    {
        Ok(_) => println!("ERROR: Duplicate nullifier should have failed!"),
        Err(e) => println!("Expected failure: {}", e),
    }

    Ok(())
}
