use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::Rpc;
use light_token::instruction::{Approve, LIGHT_TOKEN_PROGRAM_ID};
use light_token_anchor_revoke::{accounts, instruction::Revoke, ID};
use anchor_lang::system_program;
use solana_sdk::{instruction::Instruction, signature::Keypair, signer::Signer};
use test_utils::{mint_tokens, setup_test_env};

#[tokio::test]
async fn test_revoke() {
    let mut env = setup_test_env("light_token_anchor_revoke", ID).await;

    // Mint tokens first
    let mint_amount = 1_000_000u64;
    mint_tokens(&mut env.rpc, &env.payer, env.mint_pda, env.associated_token_account, mint_amount).await;

    // Approve delegate first using SDK
    let delegate = Keypair::new();
    let approve_ix = Approve {
        token_account: env.associated_token_account,
        delegate: delegate.pubkey(),
        owner: env.payer.pubkey(),
        amount: 500_000,
        fee_payer: env.payer.pubkey(),
    }
    .instruction()
    .unwrap();

    env.rpc
        .create_and_send_transaction(&[approve_ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    // Call the anchor program to revoke delegation
    let ix = Instruction {
        program_id: ID,
        accounts: accounts::RevokeAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            token_account: env.associated_token_account,
            owner: env.payer.pubkey(),
            fee_payer: env.payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(Some(true)),
        data: Revoke {}.data(),
    };

    let sig = env
        .rpc
        .create_and_send_transaction(&[ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
