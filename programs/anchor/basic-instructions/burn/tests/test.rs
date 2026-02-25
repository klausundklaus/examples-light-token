use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::Rpc;
use light_token::instruction::LIGHT_TOKEN_PROGRAM_ID;
use light_token_anchor_burn::{accounts, instruction::Burn, ID};
use anchor_lang::system_program;
use solana_sdk::{instruction::Instruction, signer::Signer};
use test_utils::{mint_tokens, setup_test_env};

#[tokio::test]
async fn test_burn() {
    let mut env = setup_test_env("light_token_anchor_burn", ID).await;

    // Mint tokens first
    let mint_amount = 1_000_000u64;
    mint_tokens(&mut env.rpc, &env.payer, env.mint_pda, env.associated_token_account, mint_amount).await;

    // Call the anchor program to burn tokens
    let burn_amount = 250_000u64;
    let ix = Instruction {
        program_id: ID,
        accounts: accounts::BurnAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            source: env.associated_token_account,
            mint: env.mint_pda,
            authority: env.payer.pubkey(),
            fee_payer: env.payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(Some(true)),
        data: Burn { amount: burn_amount }.data(),
    };

    let sig = env
        .rpc
        .create_and_send_transaction(&[ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
