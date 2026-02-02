use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::Rpc;
use light_token::instruction::LIGHT_TOKEN_PROGRAM_ID;
use light_token_anchor_freeze::{accounts, instruction::Freeze, ID};
use solana_sdk::{instruction::Instruction, signer::Signer};
use test_utils::{mint_tokens, setup_test_env_with_freeze};

#[tokio::test]
async fn test_freeze() {
    let mut env = setup_test_env_with_freeze("light_token_anchor_freeze", ID).await;

    // Mint tokens first
    mint_tokens(&mut env.rpc, &env.payer, env.mint_pda, env.associated_token_account, 1_000_000).await;

    // Call the anchor program to freeze account
    let ix = Instruction {
        program_id: ID,
        accounts: accounts::FreezeAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            token_account: env.associated_token_account,
            mint: env.mint_pda,
            freeze_authority: env.freeze_authority,
        }
        .to_account_metas(Some(true)),
        data: Freeze {}.data(),
    };

    let sig = env
        .rpc
        .create_and_send_transaction(&[ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
