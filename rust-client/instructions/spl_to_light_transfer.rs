use anchor_spl::{
    associated_token::spl_associated_token_account,
    token::{spl_token, Mint},
};
use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::{
    instruction::{
        get_associated_token_address_and_bump, get_spl_interface_pda_and_bump,
        CreateAssociatedTokenAccount, SplInterface, TransferInterface, LIGHT_TOKEN_PROGRAM_ID,
    },
    spl_interface::CreateSplInterfacePda,
};
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_system_interface::instruction as system_instruction;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(true, None)).await?;

    let payer = rpc.get_payer().insecure_clone();
    let decimals = 6u8;
    let mint_amount = 100_000u64;
    let transfer_to_light = 60_000u64;
    let transfer_back = 25_000u64;

    // 1. Create SPL mint
    let mint_seed = Keypair::new();
    let mint = mint_seed.pubkey();

    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await?;

    let create_mint_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &mint,
        mint_rent,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    let init_mint_instruction = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint,
        &payer.pubkey(),
        None,
        decimals,
    )?;

    rpc.create_and_send_transaction(
        &[create_mint_instruction, init_mint_instruction],
        &payer.pubkey(),
        &[&payer, &mint_seed],
    )
    .await?;

    // 2. Create SPL interface PDA (holds SPL tokens when transferred to Light Token)
    let (interface_pda, interface_bump) = get_spl_interface_pda_and_bump(&mint);

    let create_interface_instruction =
        CreateSplInterfacePda::new(payer.pubkey(), mint, spl_token::ID, false).instruction();

    rpc.create_and_send_transaction(&[create_interface_instruction], &payer.pubkey(), &[&payer])
        .await?;

    // 3. Create SPL associated token account
    let spl_associated_token_account =
        spl_associated_token_account::get_associated_token_address(&payer.pubkey(), &mint);

    let create_spl_associated_token_account_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint,
            &spl_token::ID,
        );

    rpc.create_and_send_transaction(&[create_spl_associated_token_account_instruction], &payer.pubkey(), &[&payer])
        .await?;

    // 4. Mint SPL tokens
    let mint_spl_instruction = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint,
        &spl_associated_token_account,
        &payer.pubkey(),
        &[],
        mint_amount,
    )?;

    rpc.create_and_send_transaction(&[mint_spl_instruction], &payer.pubkey(), &[&payer])
        .await?;

    // 5. Create Light Token associated token account
    let (light_associated_token_account, _) = get_associated_token_address_and_bump(&payer.pubkey(), &mint);

    let create_light_associated_token_account_instruction =
        CreateAssociatedTokenAccount::new(payer.pubkey(), payer.pubkey(), mint).instruction()?;

    rpc.create_and_send_transaction(&[create_light_associated_token_account_instruction], &payer.pubkey(), &[&payer])
        .await?;

    // 6. Transfer SPL → Light Token (source/destination owner program IDs determine token standards: SPL, Token 2022, or Light)
    let spl_interface = SplInterface {
        mint,
        spl_token_program: spl_token::ID,
        spl_interface_pda: interface_pda,
        spl_interface_pda_bump: interface_bump,
    };

    let spl_to_light_instruction = TransferInterface {
        source: spl_associated_token_account,
        destination: light_associated_token_account,
        amount: transfer_to_light,
        decimals,
        authority: payer.pubkey(),
        payer: payer.pubkey(),
        mint,
        spl_interface: Some(spl_interface),
        source_owner: spl_token::ID,
        destination_owner: LIGHT_TOKEN_PROGRAM_ID,
    }
    .instruction()?;

    rpc.create_and_send_transaction(&[spl_to_light_instruction], &payer.pubkey(), &[&payer])
        .await?;

    // 7. Transfer Light Token → SPL (source/destination owner program IDs determine token standards: SPL, Token 2022, or Light)
    let light_to_spl_instruction = TransferInterface {
        source: light_associated_token_account,
        destination: spl_associated_token_account,
        amount: transfer_back,
        decimals,
        authority: payer.pubkey(),
        payer: payer.pubkey(),
        mint,
        spl_interface: Some(spl_interface),
        source_owner: LIGHT_TOKEN_PROGRAM_ID,
        destination_owner: spl_token::ID,
    }
    .instruction()?;

    let sig = rpc
        .create_and_send_transaction(&[light_to_spl_instruction], &payer.pubkey(), &[&payer])
        .await?;

    let data = rpc
        .get_account(light_associated_token_account)
        .await?
        .ok_or("Account not found")?;
    let token = light_token_interface::state::Token::deserialize(&mut &data.data[..])?;
    println!("Balance: {} Tx: {sig}", token.amount);

    Ok(())
}
