use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::Rpc;
use light_token::instruction::{rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID};
use light_token_anchor_close::{accounts, instruction::CloseAccount, ID};
use solana_sdk::{instruction::Instruction, signer::Signer};
use test_utils::setup_test_env;

#[tokio::test]
async fn test_close() {
    let mut env = setup_test_env("light_token_anchor_close", ID).await;

    // Associated token account must be empty to close (no mint_tokens call).

    // Call the anchor program to close account
    let rent_sponsor = rent_sponsor_pda();

    let ix = Instruction {
        program_id: ID,
        accounts: accounts::CloseAccountAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            account: env.associated_token_account,
            destination: env.payer.pubkey(),
            owner: env.payer.pubkey(),
            rent_sponsor,
        }
        .to_account_metas(Some(true)),
        data: CloseAccount {}.data(),
    };

    let sig = env
        .rpc
        .create_and_send_transaction(&[ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    println!("Tx: {}", sig);
}
