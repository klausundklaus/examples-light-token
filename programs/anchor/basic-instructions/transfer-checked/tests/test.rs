use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::Rpc;
use light_token::instruction::LIGHT_TOKEN_PROGRAM_ID;
use light_token_anchor_transfer_checked::{accounts, instruction::TransferChecked, ID};
use anchor_lang::system_program;
use solana_sdk::{instruction::Instruction, signature::Keypair, signer::Signer};
use basic_instructions_test_utils::{create_associated_token_account_for_owner, mint_tokens, setup_test_env};

#[tokio::test]
async fn test_transfer_checked() {
    let mut env = setup_test_env("light_token_anchor_transfer_checked", ID).await;
    mint_tokens(&mut env.rpc, &env.payer, env.mint_pda, env.associated_token_account, 1_000_000).await;

    // Create destination associated token account for recipient
    let recipient = Keypair::new();
    let dest_associated_token_account =
        create_associated_token_account_for_owner(&mut env.rpc, &env.payer, &recipient.pubkey(), &env.mint_pda).await;

    // TransferChecked validates decimals match the mint's decimals.
    // Only use for Light->Light transfers.
    // Use TransferInterface for all other transfers (Light, SPL or Token-2022).
    let transfer_amount = 100_000u64;
    let decimals = 9u8;

    let ix = Instruction {
        program_id: ID,
        accounts: accounts::TransferCheckedAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            source: env.associated_token_account,
            mint: env.mint_pda,
            destination: dest_associated_token_account,
            authority: env.payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(Some(true)),
        data: TransferChecked {
            amount: transfer_amount,
            decimals,
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
