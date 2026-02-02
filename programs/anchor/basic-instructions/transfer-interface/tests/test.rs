use anchor_lang::system_program;
use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::token::{spl_token, Mint};
use light_program_test::Rpc;
use solana_sdk::program_pack::Pack as _;
use light_token::instruction::{
    derive_token_ata, CreateAssociatedTokenAccount, LIGHT_TOKEN_PROGRAM_ID,
};
use light_token::spl_interface::{find_spl_interface_pda_with_index, CreateSplInterfacePda};
use light_token_anchor_transfer_interface::{accounts, instruction::Transfer, ID};
use light_token_types::CPI_AUTHORITY_PDA;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use test_utils::{create_associated_token_account_for_owner, mint_tokens, setup_test_env};

#[tokio::test]
async fn test_transfer() {
    let mut env = setup_test_env("light_token_anchor_transfer_interface", ID).await;
    mint_tokens(&mut env.rpc, &env.payer, env.mint_pda, env.associated_token_account, 1_000_000).await;

    // Create destination associated token account for recipient
    let recipient = Keypair::new();
    let dest_associated_token_account =
        create_associated_token_account_for_owner(&mut env.rpc, &env.payer, &recipient.pubkey(), &env.mint_pda).await;

    // Transfers tokens between accounts (SPL, Token-2022, or Light) in a single call.
    let transfer_amount = 100_000u64;
    let decimals = 9u8;
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    let ix = Instruction {
        program_id: ID,
        accounts: accounts::TransferAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            source: env.associated_token_account,
            destination: dest_associated_token_account,
            authority: env.payer.pubkey(),
            payer: env.payer.pubkey(),
            cpi_authority: cpi_authority_pda,
            system_program: system_program::ID,
            mint: None,
            spl_token_program: None,
            spl_interface_pda: None,
        }
        .to_account_metas(Some(true)),
        data: Transfer {
            amount: transfer_amount,
            decimals,
            spl_interface_pda_bump: None,
        }
        .data(),
    };

    let sig = env
        .rpc
        .create_and_send_transaction(&[ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}

#[tokio::test]
async fn test_transfer_spl_to_light() {
    let mut env = setup_test_env("light_token_anchor_transfer_interface", ID).await;

    let payer = env.payer.insecure_clone();
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // 1. Create SPL mint
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();
    let decimals = 9u8;

    let mint_rent = env
        .rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();

    let create_mint_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint,
        mint_rent,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    let initialize_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint,
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    env.rpc
        .create_and_send_transaction(
            &[create_mint_account_ix, initialize_mint_ix],
            &payer.pubkey(),
            &[&payer, &mint_keypair],
        )
        .await
        .unwrap();

    // 2. Create SPL token pool (spl_interface_pda)
    let create_pool_ix =
        CreateSplInterfacePda::new(payer.pubkey(), mint, spl_token::ID, false).instruction();

    env.rpc
        .create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // 3. Create SPL token account and mint tokens
    let spl_account_keypair = Keypair::new();
    let spl_account = spl_account_keypair.pubkey();

    let spl_account_rent = env
        .rpc
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
        .await
        .unwrap();

    let create_spl_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &spl_account,
        spl_account_rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::ID,
    );

    let init_spl_account_ix = spl_token::instruction::initialize_account(
        &spl_token::ID,
        &spl_account,
        &mint,
        &payer.pubkey(),
    )
    .unwrap();

    let mint_amount = 1_000_000u64;
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint,
        &spl_account,
        &payer.pubkey(),
        &[],
        mint_amount,
    )
    .unwrap();

    env.rpc
        .create_and_send_transaction(
            &[create_spl_account_ix, init_spl_account_ix, mint_to_ix],
            &payer.pubkey(),
            &[&payer, &spl_account_keypair],
        )
        .await
        .unwrap();

    // 4. Create Light ATA for destination
    let recipient = Keypair::new();
    let (dest_ata, _) = derive_token_ata(&recipient.pubkey(), &mint);
    let create_ata_ix = CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), mint)
        .instruction()
        .unwrap();

    env.rpc
        .create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // 5. Transfer SPL tokens to Light ATA via transfer-interface
    let transfer_amount = 100_000u64;
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);

    let ix = Instruction {
        program_id: ID,
        accounts: accounts::TransferAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            source: spl_account,
            destination: dest_ata,
            authority: payer.pubkey(),
            payer: payer.pubkey(),
            cpi_authority: cpi_authority_pda,
            system_program: system_program::ID,
            mint: Some(mint),
            spl_token_program: Some(spl_token::ID),
            spl_interface_pda: Some(spl_interface_pda),
        }
        .to_account_metas(Some(true)),
        data: Transfer {
            amount: transfer_amount,
            decimals,
            spl_interface_pda_bump: Some(spl_interface_pda_bump),
        }
        .data(),
    };

    let sig = env
        .rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
