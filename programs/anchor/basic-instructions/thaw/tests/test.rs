use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::Rpc;
use light_token::instruction::{Freeze, LIGHT_TOKEN_PROGRAM_ID};
use light_token_anchor_thaw::{accounts, instruction::Thaw, ID};
use solana_sdk::{instruction::Instruction, signer::Signer};
use basic_instructions_test_utils::{mint_tokens, setup_test_env_with_freeze};

#[tokio::test]
async fn test_thaw() {
    let mut env = setup_test_env_with_freeze("light_token_anchor_thaw", ID).await;

    // Mint tokens first
    mint_tokens(&mut env.rpc, &env.payer, env.mint_pda, env.associated_token_account, 1_000_000).await;

    // Freeze account first using SDK
    let freeze_ix = Freeze {
        token_account: env.associated_token_account,
        mint: env.mint_pda,
        freeze_authority: env.freeze_authority,
    }
    .instruction()
    .unwrap();

    env.rpc
        .create_and_send_transaction(&[freeze_ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    // Call the anchor program to thaw account
    let ix = Instruction {
        program_id: ID,
        accounts: accounts::ThawAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            token_account: env.associated_token_account,
            mint: env.mint_pda,
            freeze_authority: env.freeze_authority,
        }
        .to_account_metas(Some(true)),
        data: Thaw {}.data(),
    };

    let sig = env
        .rpc
        .create_and_send_transaction(&[ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
