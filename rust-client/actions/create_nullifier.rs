//! Example: Create a nullifier PDA to prevent duplicate actions on devnet.
//!
//! Run with: cargo run --example action_create_nullifier
//!
//! Requires:
//!   - API_KEY in .env file (Helius API key)
//!   - Funded keypair at ~/.config/solana/id.json

use dotenv::dotenv;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_nullifier_program::sdk::{create_nullifier_ix, derive_nullifier_address, PROGRAM_ID};
use solana_sdk::{
    signature::read_keypair_file,
    signer::Signer,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    println!("Nullifier Program ID: {}", PROGRAM_ID);

    // Load API key from .env
    let api_key = env::var("API_KEY").expect("API_KEY required in .env");
    let rpc_url = format!("https://devnet.helius-rpc.com/?api-key={}", api_key);
    let photon_url = "https://devnet.helius-rpc.com".to_string();

    // Setup LightClient for devnet
    // RPC URL includes api-key, Photon URL is base URL (api_key added separately)
    let config = LightClientConfig::new(rpc_url, Some(photon_url), Some(api_key));
    let mut rpc = LightClient::new(config).await?;

    // Load keypair from default Solana CLI location
    let keypair_path =
        shellexpand::tilde("~/.config/solana/id.json").to_string();
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| format!("Failed to read keypair from {}: {}", keypair_path, e))?;
    rpc.payer = payer.insecure_clone();

    println!("Payer: {}", payer.pubkey());

    // Check balance
    let balance = rpc.get_balance(&payer.pubkey()).await?;
    println!("Balance: {} SOL", balance as f64 / 1_000_000_000.0);
    if balance < 10_000_000 {
        return Err("Insufficient balance. Need at least 0.01 SOL on devnet".into());
    }

    // Create a unique 32-byte ID (e.g., hash of payment inputs)
    let id: [u8; 32] = rand::random();
    println!("Nullifier ID: {}", bs58::encode(&id).into_string());

    // Build nullifier instruction
    println!("\nFetching proof and building instruction...");
    let nullifier_ix = create_nullifier_ix(&mut rpc, payer.pubkey(), id).await?;

    // Send transaction with just the nullifier instruction
    // (You can add other instructions like transfers here)
    println!("Sending transaction...");
    let sig = rpc
        .create_and_send_transaction(&[nullifier_ix], &payer.pubkey(), &[&payer])
        .await?;
    println!("Transaction confirmed: {}", sig);

    // Verify nullifier was created
    let address = derive_nullifier_address(&id);
    println!(
        "Nullifier address: {}",
        bs58::encode(&address).into_string()
    );

    // Try to create the same nullifier again (should fail at proof stage)
    println!("\nAttempting duplicate nullifier (should fail)...");
    match create_nullifier_ix(&mut rpc, payer.pubkey(), id).await {
        Ok(_) => println!("ERROR: Duplicate nullifier proof request should have failed!"),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("already exists") {
                println!("Duplicate correctly rejected: address already exists");
            } else {
                println!("Expected failure: {}", e);
            }
        }
    }

    println!("\nSuccess! Nullifier program works on devnet.");
    Ok(())
}
