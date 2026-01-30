use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{get_create_accounts_proof, InitializeRentFreeConfig};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{
    derive_token_ata, find_mint_address, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_signer::Signer;
use create_and_transfer::{TransferParams, ID};

/// Creates a compressed mint. Returns (mint_pda, mint_seed_keypair).
async fn setup_create_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: solana_pubkey::Pubkey,
    decimals: u8,
) -> (solana_pubkey::Pubkey, Keypair) {
    use light_token::instruction::{CreateMint, CreateMintParams, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = find_mint_address(&mint_seed.pubkey());

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let params = CreateMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: DEFAULT_RENT_PAYMENT,
        write_top_up: DEFAULT_WRITE_TOP_UP,
    };

    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    (mint, mint_seed)
}

/// Mints tokens to the specified associated token account.
async fn mint_tokens(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_pda: solana_pubkey::Pubkey,
    associated_token_account: solana_pubkey::Pubkey,
    amount: u64,
) {
    use light_token::instruction::MintTo;

    let mint_to_ix = MintTo {
        mint: mint_pda,
        destination: associated_token_account,
        amount,
        authority: payer.pubkey(),
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[mint_to_ix], &payer.pubkey(), &[payer])
        .await
        .unwrap();
}

#[tokio::test]
async fn test_transfer() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("create_and_transfer", ID)]))
        .with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &ID);

    let (init_config_ix, _config_pda) = InitializeRentFreeConfig::new(
        &ID,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let (mint_pda, _mint_seed) = setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(), // mint_authority
        9,              // decimals
    )
    .await;

    println!("Mint created at: {}", mint_pda);

    let sender = Keypair::new();
    let (sender_associated_token_account, _sender_associated_token_account_bump) = derive_token_ata(&sender.pubkey(), &mint_pda);

    let create_sender_associated_token_account_ix =
        light_token::instruction::CreateAssociatedTokenAccount::new(
            payer.pubkey(),
            sender.pubkey(),
            mint_pda,
        )
        .instruction()
        .unwrap();

    rpc.create_and_send_transaction(&[create_sender_associated_token_account_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let mint_amount: u64 = 1_000_000_000;
    mint_tokens(&mut rpc, &payer, mint_pda, sender_associated_token_account, mint_amount).await;

    println!("Minted {} tokens to sender: {}", mint_amount, sender_associated_token_account);

    let recipient = Keypair::new();
    let (recipient_associated_token_account, recipient_associated_token_account_bump) = derive_token_ata(&recipient.pubkey(), &mint_pda);

    let transfer_proof_result = get_create_accounts_proof(&rpc, &ID, vec![])
        .await
        .unwrap();

    let transfer_accounts = create_and_transfer::accounts::Transfer {
        payer: payer.pubkey(),
        authority: sender.pubkey(),
        mint: mint_pda,
        source: sender_associated_token_account,
        recipient: recipient.pubkey(),
        destination: recipient_associated_token_account,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
    };

    let transfer_amount: u64 = 500_000_000; // 0.5 tokens
    let decimals: u8 = 9;

    let transfer_ix = Instruction {
        program_id: ID,
        accounts: [
            transfer_accounts.to_account_metas(None),
            transfer_proof_result.remaining_accounts,
        ]
        .concat(),
        data: create_and_transfer::instruction::Transfer {
            params: TransferParams {
                create_accounts_proof: transfer_proof_result.create_accounts_proof,
                dest_associated_token_account_bump: recipient_associated_token_account_bump,
                amount: transfer_amount,
                decimals,
            },
        }
        .data(),
    };

    let sig = rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &sender])
        .await
        .unwrap();

    println!("Transfer Tx: {}", sig);

    use light_token_interface::state::Token;

    let recipient_account = rpc.get_account(recipient_associated_token_account).await.unwrap().unwrap();
    let recipient_token: Token =
        borsh::BorshDeserialize::deserialize(&mut &recipient_account.data[..]).unwrap();

    assert_eq!(recipient_token.amount, transfer_amount);
    assert_eq!(recipient_token.owner, recipient.pubkey().to_bytes());

    println!(
        "Recipient balance: {}, owner: {}",
        recipient_token.amount,
        recipient.pubkey()
    );

    let sender_account = rpc.get_account(sender_associated_token_account).await.unwrap().unwrap();
    let sender_token: Token =
        borsh::BorshDeserialize::deserialize(&mut &sender_account.data[..]).unwrap();

    assert_eq!(sender_token.amount, mint_amount - transfer_amount);
    println!("Sender remaining balance: {}", sender_token.amount);
}
