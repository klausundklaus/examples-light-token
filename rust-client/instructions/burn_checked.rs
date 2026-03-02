use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_token::instruction::BurnChecked;
use rust_client::{setup, SetupContext};
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup creates mint and associated token account with tokens
    let SetupContext {
        mut rpc,
        payer,
        mint,
        associated_token_account,
        decimals,
        ..
    } = setup().await;

    let burn_amount = 400_000u64;

    // BurnChecked validates decimals match the mint's decimals
    let burn_instruction = BurnChecked {
        source: associated_token_account,
        mint,
        amount: burn_amount,
        decimals,
        authority: payer.pubkey(),
        fee_payer: payer.pubkey(),
    }
    .instruction()?;

    let sig = rpc
        .create_and_send_transaction(&[burn_instruction], &payer.pubkey(), &[&payer])
        .await?;

    let data = rpc.get_account(associated_token_account).await?.ok_or("Account not found")?;
    let token = light_token_interface::state::Token::deserialize(&mut &data.data[..])?;
    println!("Balance: {} Tx: {sig}", token.amount);

    Ok(())
}
