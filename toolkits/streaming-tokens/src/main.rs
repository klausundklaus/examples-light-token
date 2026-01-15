use futures::StreamExt;
use helius_laserstream::{subscribe, LaserstreamConfig};

const LIGHT_TOKEN_PROGRAM_ID: &str = "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("API_KEY")?;
    let endpoint = "https://laserstream-devnet-ewr.helius-rpc.com".to_string();

    let config = LaserstreamConfig::new(endpoint, api_key);

    let request = helius_laserstream::grpc::SubscribeRequest {
        transactions: [(
            "light_token".to_string(),
            helius_laserstream::grpc::SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                account_include: vec![LIGHT_TOKEN_PROGRAM_ID.to_string()],
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    };

    let (stream, _handle) = subscribe(config, request);
    tokio::pin!(stream);

    println!("Connected to Laserstream (devnet) ...\n");
    println!("Listening for transactions ...\n");

    while let Some(update) = stream.next().await {
        match update {
            Ok(msg) => {
                if let Some(update_oneof) = msg.update_oneof {
                    if let helius_laserstream::grpc::subscribe_update::UpdateOneof::Transaction(tx_info) = update_oneof {
                        if let Some(tx_wrapper) = &tx_info.transaction {
                            let sig = bs58::encode(&tx_wrapper.signature).into_string();
                            println!("Transaction: https://explorer.solana.com/tx/{}?cluster=devnet\n", sig);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Stream error: {:?}", e);
            }
        }
    }

    Ok(())
}