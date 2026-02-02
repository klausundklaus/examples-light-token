use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::Rpc;
use light_token::instruction::LIGHT_TOKEN_PROGRAM_ID;
use light_token_anchor_mint_to::{accounts, instruction::MintTo, ID};
use anchor_lang::system_program;
use solana_sdk::{instruction::Instruction, signer::Signer};
use test_utils::setup_test_env;

#[tokio::test(flavor = "multi_thread")]
async fn test_mint_to() {
    let mut env = setup_test_env("light_token_anchor_mint_to", ID).await;

    // No mint_tokens call - the test IS minting tokens via CPI.
    let amount = 1_000_000u64;
    let ix = Instruction {
        program_id: ID,
        accounts: accounts::MintToAccounts {
            light_token_program: LIGHT_TOKEN_PROGRAM_ID,
            mint: env.mint_pda,
            destination: env.associated_token_account,
            authority: env.payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(Some(true)),
        data: MintTo { amount }.data(),
    };

    env.rpc
        .create_and_send_transaction(&[ix], &env.payer.pubkey(), &[&env.payer])
        .await
        .unwrap();

    // Verify the account exists and has data
    let associated_token_account_data = env.rpc.get_account(env.associated_token_account).await.unwrap().unwrap();
    assert!(!associated_token_account_data.data.is_empty(), "Associated token account should have data");
}
