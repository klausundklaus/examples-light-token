//! Create nullifier example. Run: cargo run --example action_create_nullifier

use dotenv::dotenv;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_nullifier_program::sdk::{create_nullifier_ix, derive_nullifier_address};
use solana_sdk::{signature::read_keypair_file, signer::Signer};
use std::env;

async fn get_client() -> Result<LightClient, Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = env::var("API_KEY").expect("API_KEY required in .env");
    let rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);
    let photon_url = "https://mainnet.helius-rpc.com".to_string();
    let config = LightClientConfig::new(rpc_url, Some(photon_url), Some(api_key));
    let mut rpc = LightClient::new(config).await?;

    let keypair_path = shellexpand::tilde("~/.config/solana/id.json").to_string();
    let payer =
        read_keypair_file(&keypair_path).map_err(|e| format!("Failed to read keypair: {}", e))?;
    rpc.payer = payer;
    Ok(rpc)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rpc = get_client().await?;
    let payer = rpc.get_payer().insecure_clone();

    // must be identifier, eg, nonce/UUID/hash of payment info
    let id: [u8; 32] = rand::random();
    let nullifier_ix = create_nullifier_ix(&mut rpc, payer.pubkey(), id).await?;

    let sig = rpc
        .create_and_send_transaction(&[nullifier_ix], &payer.pubkey(), &[&payer])
        .await?;
    println!("{}", sig);

    let _address = derive_nullifier_address(&id);

    // Double spend should fail
    assert!(create_nullifier_ix(&mut rpc, payer.pubkey(), id)
        .await
        .is_err());

    Ok(())
}
