use borsh::BorshDeserialize;
use futures::StreamExt;
use helius_laserstream::grpc::subscribe_request_filter_accounts_filter::Filter;
use helius_laserstream::grpc::subscribe_request_filter_accounts_filter_memcmp::Data;
use helius_laserstream::grpc::{
    subscribe_update::UpdateOneof, SubscribeRequestFilterAccounts,
    SubscribeRequestFilterAccountsFilter, SubscribeRequestFilterAccountsFilterMemcmp,
    SubscribeRequestFilterTransactions, SubscribeUpdateTransactionInfo,
};
use helius_laserstream::{subscribe, LaserstreamConfig};
use light_token_interface::instructions::extensions::ExtensionInstructionData;
use light_token_interface::instructions::mint_action::MintActionCompressedInstructionData;
use light_token_interface::state::{ExtensionStruct, Mint, Token};

const LIGHT_TOKEN_PROGRAM_ID: &str = "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m";

/// Base SPL Token Account size (without extensions).
/// Light ATAs without extensions are exactly 165 bytes.
const TOKEN_ACCOUNT_SIZE: u64 = 165;

/// Byte offset of `account_type` in a Mint account.
/// BaseMint (82) + MintMetadata (67) + reserved (16) = 165.
const ACCOUNT_TYPE_OFFSET: u64 = 165;

/// MintAction instruction discriminator in the Light Token Program.
const MINT_ACTION_DISCRIMINATOR: u8 = 103;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    dotenvy::dotenv().ok();

    let api_key = std::env::var("LASERSTREAM_API_KEY")?;
    let endpoint = "https://laserstream-devnet-ewr.helius-rpc.com".to_string();

    let config = LaserstreamConfig::new(endpoint, api_key);

    let mut request = helius_laserstream::grpc::SubscribeRequest::default();

    // Subscribe to Token (ATA) accounts: 165-byte accounts owned by Light Token Program.
    // ATAs without extensions are exactly 165 bytes (the base SPL Token Account layout).
    request.accounts.insert(
        "light_atas".to_string(),
        SubscribeRequestFilterAccounts {
            owner: vec![LIGHT_TOKEN_PROGRAM_ID.to_string()],
            filters: vec![SubscribeRequestFilterAccountsFilter {
                filter: Some(Filter::Datasize(TOKEN_ACCOUNT_SIZE)),
            }],
            nonempty_txn_signature: Some(true),
            ..Default::default()
        },
    );

    // Subscribe to Mint accounts: account_type == 1 at byte offset 165.
    // Mint accounts are variable-sized (extensions), so use memcmp instead of datasize.
    request.accounts.insert(
        "light_mints".to_string(),
        SubscribeRequestFilterAccounts {
            owner: vec![LIGHT_TOKEN_PROGRAM_ID.to_string()],
            filters: vec![SubscribeRequestFilterAccountsFilter {
                filter: Some(Filter::Memcmp(SubscribeRequestFilterAccountsFilterMemcmp {
                    offset: ACCOUNT_TYPE_OFFSET,
                    data: Some(Data::Bytes(vec![1])),
                })),
            }],
            nonempty_txn_signature: Some(true),
            ..Default::default()
        },
    );

    // Subscribe to transactions involving the Light Token Program.
    // Catches light mint creation (MintAction, discriminator 103).
    request.transactions.insert(
        "light_mint_txns".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![LIGHT_TOKEN_PROGRAM_ID.to_string()],
            ..Default::default()
        },
    );

    let (stream, _handle) = subscribe(config, request);
    tokio::pin!(stream);

    println!("Listening for light-token accounts (devnet)...\n");

    while let Some(update) = stream.next().await {
        match update {
            Ok(msg) => {
                let filters = msg.filters.clone();
                let is_mint = filters.iter().any(|f| f == "light_mints");

                match msg.update_oneof {
                    Some(UpdateOneof::Transaction(tx_update)) => {
                        if let Some(tx_info) = tx_update.transaction {
                            handle_transaction(&tx_info);
                        }
                    }
                    Some(UpdateOneof::Account(account_update)) => {
                        if let Some(account_info) = account_update.account {
                            let pubkey = bs58::encode(&account_info.pubkey).into_string();
                            let tx_sig = account_info
                                .txn_signature
                                .as_ref()
                                .map(|s| bs58::encode(s).into_string())
                                .unwrap_or_default();

                            if is_mint {
                                print_mint(&pubkey, &tx_sig, &account_info.data);
                            } else {
                                print_token(&pubkey, &tx_sig, &account_info.data);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Stream error: {:?}", e);
            }
        }
    }

    Ok(())
}

/// Scans transaction instructions for MintAction (discriminator 103) and prints
/// light mint creation details.
fn handle_transaction(tx_info: &SubscribeUpdateTransactionInfo) {
    let sig = bs58::encode(&tx_info.signature).into_string();

    let Some(tx) = &tx_info.transaction else {
        return;
    };
    let Some(msg) = &tx.message else {
        return;
    };

    let program_id_bytes = bs58::decode(LIGHT_TOKEN_PROGRAM_ID)
        .into_vec()
        .unwrap();

    for ix in &msg.instructions {
        let prog_key = &msg.account_keys[ix.program_id_index as usize];
        if prog_key != &program_id_bytes {
            continue;
        }
        if ix.data.first() != Some(&MINT_ACTION_DISCRIMINATOR) {
            continue;
        }

        match MintActionCompressedInstructionData::deserialize(&mut &ix.data[1..]) {
            Ok(mint_ix) if mint_ix.create_mint.is_some() => {
                println!("light-token mint created");
                println!("  tx: {}", sig);

                if let Some(mint_data) = &mint_ix.mint {
                    println!("  decimals: {}", mint_data.decimals);
                    println!("  supply: {}", mint_data.supply);

                    if let Some(auth) = &mint_data.mint_authority {
                        println!(
                            "  mint_authority: {}",
                            bs58::encode(auth.to_bytes()).into_string()
                        );
                    }
                    if let Some(auth) = &mint_data.freeze_authority {
                        println!(
                            "  freeze_authority: {}",
                            bs58::encode(auth.to_bytes()).into_string()
                        );
                    }

                    if let Some(exts) = &mint_data.extensions {
                        for ext in exts {
                            if let ExtensionInstructionData::TokenMetadata(meta) = ext {
                                println!("  name: {}", String::from_utf8_lossy(&meta.name));
                                println!("  symbol: {}", String::from_utf8_lossy(&meta.symbol));
                                println!("  uri: {}", String::from_utf8_lossy(&meta.uri));
                            }
                        }
                    }
                }
                println!();
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to parse MintAction ix: {}", e);
            }
        }
    }
}

/// Deserializes and prints a Mint account.
fn print_mint(pubkey: &str, tx_sig: &str, data: &[u8]) {
    match Mint::deserialize(&mut &*data) {
        Ok(mint) => {
            println!("Mint: {}", pubkey);
            println!("  tx: {}", tx_sig);
            println!("  decimals: {}", mint.base.decimals);
            println!("  supply: {}", mint.base.supply);

            if let Some(authority) = &mint.base.mint_authority {
                println!(
                    "  mint_authority: {}",
                    bs58::encode(authority.to_bytes()).into_string()
                );
            }

            if let Some(authority) = &mint.base.freeze_authority {
                println!(
                    "  freeze_authority: {}",
                    bs58::encode(authority.to_bytes()).into_string()
                );
            }

            if let Some(extensions) = &mint.extensions {
                for ext in extensions {
                    if let ExtensionStruct::TokenMetadata(meta) = ext {
                        println!("  name: {}", String::from_utf8_lossy(&meta.name));
                        println!("  symbol: {}", String::from_utf8_lossy(&meta.symbol));
                        println!("  uri: {}", String::from_utf8_lossy(&meta.uri));
                    }
                }
            }

            println!();
        }
        Err(e) => {
            eprintln!("Failed to deserialize mint {}: {}", pubkey, e);
        }
    }
}

/// Deserializes and prints a Token (ATA) account.
fn print_token(pubkey: &str, tx_sig: &str, data: &[u8]) {
    match Token::deserialize(&mut &*data) {
        Ok(token) => {
            let mint = bs58::encode(token.mint.to_bytes()).into_string();
            let owner = bs58::encode(token.owner.to_bytes()).into_string();

            println!("ATA: {}", pubkey);
            println!("  tx: {}", tx_sig);
            println!("  mint: {}", mint);
            println!("  owner: {}", owner);
            println!("  amount: {}", token.amount);
            println!("  state: {:?}", token.state);

            if let Some(delegate) = &token.delegate {
                let del = bs58::encode(delegate.to_bytes()).into_string();
                println!("  delegate: {}", del);
                println!("  delegated_amount: {}", token.delegated_amount);
            }

            if let Some(close_auth) = &token.close_authority {
                let ca = bs58::encode(close_auth.to_bytes()).into_string();
                println!("  close_authority: {}", ca);
            }

            println!();
        }
        Err(e) => {
            eprintln!("Failed to deserialize ATA {}: {}", pubkey, e);
        }
    }
}
