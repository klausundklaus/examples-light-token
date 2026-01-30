use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_token::instruction::MintToChecked;
use rust_client::{setup_empty_associated_token_account, SetupContext};
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup creates mint and empty associated token account
    let SetupContext {
        mut rpc,
        payer,
        mint,
        associated_token_account,
        decimals,
        ..
    } = setup_empty_associated_token_account().await;

    let mint_amount = 1_000_000_000u64;

    // MintToChecked validates decimals match the mint's decimals
    let mint_to_instruction = MintToChecked {
        mint,
        destination: associated_token_account,
        amount: mint_amount,
        decimals,
        authority: payer.pubkey(),
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()?;

    let sig = rpc
        .create_and_send_transaction(&[mint_to_instruction], &payer.pubkey(), &[&payer])
        .await?;

    let data = rpc.get_account(associated_token_account).await?.ok_or("Account not found")?;
    let token = light_token_interface::state::Token::deserialize(&mut &data.data[..])?;
    println!("Balance: {} Tx: {sig}", token.amount);

    Ok(())
}
