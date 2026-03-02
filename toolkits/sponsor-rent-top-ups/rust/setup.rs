// Test setup: creates an SPL mint, funds the sponsor, wraps into Light.
// In production the mint already exists (e.g. USDC) and the sender
// already holds Light tokens.

use anchor_spl::{
    associated_token::spl_associated_token_account,
    token::{spl_token, Mint as SPLMint},
};
use light_client::rpc::Rpc;
use light_program_test::LightProgramTest;
use light_token::{
    instruction::{
        get_associated_token_address, CreateAssociatedTokenAccount, SplInterface,
        TransferInterface, LIGHT_TOKEN_PROGRAM_ID,
    },
    spl_interface::CreateSplInterfacePda,
};
#[allow(deprecated)]
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction};

pub struct SetupResult {
    pub mint: Pubkey,
    pub sender_ata: Pubkey,
}

async fn setup_spl_mint(rpc: &mut LightProgramTest, payer: &Keypair, decimals: u8) -> Pubkey {
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();

    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(SPLMint::LEN)
        .await
        .unwrap();

    let create_mint_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &mint,
        mint_rent,
        SPLMint::LEN as u64,
        &spl_token::ID,
    );

    let init_mint_instruction = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint,
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint_instruction, init_mint_instruction],
        &payer.pubkey(),
        &[payer, &mint_keypair],
    )
    .await
    .unwrap();

    let create_interface_instruction =
        CreateSplInterfacePda::new(payer.pubkey(), mint, spl_token::ID, false).instruction();

    rpc.create_and_send_transaction(&[create_interface_instruction], &payer.pubkey(), &[payer])
        .await
        .unwrap();

    mint
}

async fn setup_spl_associated_token_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let associated_token_account =
        spl_associated_token_account::get_associated_token_address(owner, mint);

    let create_ata_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            owner,
            mint,
            &spl_token::ID,
        );

    let mut instructions = vec![create_ata_instruction];

    if amount > 0 {
        let mint_instruction = spl_token::instruction::mint_to(
            &spl_token::ID,
            mint,
            &associated_token_account,
            &payer.pubkey(),
            &[],
            amount,
        )
        .unwrap();
        instructions.push(mint_instruction);
    }

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();

    associated_token_account
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
    let (interface_pda, interface_bump) =
        light_token::spl_interface::find_spl_interface_pda_with_index(&mint, 0, false);

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
        mint,
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
