use anchor_spl::{
    associated_token::spl_associated_token_account,
    token::{spl_token, Mint as SPLMint},
};
use light_client::{
    indexer::{AddressWithTree, Indexer},
    rpc::Rpc,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::{
    instruction::{
        derive_mint_compressed_address, derive_token_ata, find_mint_address, Approve,
        CreateAssociatedTokenAccount, CreateMint, CreateMintParams, Freeze, MintTo,
    },
    spl_interface::CreateSplInterfacePda,
};
#[allow(deprecated)]
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction};

pub async fn setup_mint_with_tokens(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    decimals: u8,
    recipients: Vec<(u64, Pubkey)>,
) -> (Pubkey, Vec<Pubkey>) {
    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compression_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree.tree);
    let mint = find_mint_address(&mint_seed.pubkey()).0;

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
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
        bump: find_mint_address(&mint_seed.pubkey()).1,
        freeze_authority,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };

    let create_mint_instruction = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    )
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[create_mint_instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    if recipients.is_empty() {
        return (mint, vec![]);
    }

    let mut associated_token_account_pubkeys = Vec::with_capacity(recipients.len());

    for (amount, owner) in &recipients {
        let associated_token_account_address = derive_token_ata(owner, &mint);
        associated_token_account_pubkeys.push(associated_token_account_address);

        let create_associated_token_account_instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, mint)
            .instruction()
            .unwrap();

        rpc.create_and_send_transaction(&[create_associated_token_account_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();

        if *amount > 0 {
            let mint_to_instruction = MintTo {
                mint,
                destination: associated_token_account_address,
                amount: *amount,
                authority: mint_authority,
                fee_payer: payer.pubkey(),
            }
            .instruction()
            .unwrap();

            rpc.create_and_send_transaction(&[mint_to_instruction], &payer.pubkey(), &[payer])
                .await
                .unwrap();
        }
    }

    (mint, associated_token_account_pubkeys)
}

pub async fn setup_spl_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    decimals: u8,
) -> Pubkey {
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

pub async fn setup_spl_associated_token_account(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let associated_token_account = spl_associated_token_account::get_associated_token_address(owner, mint);

    let create_associated_token_account_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            owner,
            mint,
            &spl_token::ID,
        );

    let mut instructions = vec![create_associated_token_account_instruction];

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

pub struct SetupContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint: Pubkey,
    pub associated_token_account: Pubkey,
    pub delegate: Keypair,
    pub decimals: u8,
}

pub async fn setup() -> SetupContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let initial_amount = 1_000_000u64;

    let (mint, associated_token_accounts) = setup_mint_with_tokens(
        &mut rpc,
        &payer,
        payer.pubkey(),
        Some(payer.pubkey()),
        9,
        vec![(initial_amount, payer.pubkey())],
    )
    .await;

    let associated_token_account = associated_token_accounts[0];
    let delegate = Keypair::new();

    let approve_instruction = Approve {
        token_account: associated_token_account,
        delegate: delegate.pubkey(),
        owner: payer.pubkey(),
        amount: initial_amount,
        fee_payer: payer.pubkey(),
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[approve_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    SetupContext {
        rpc,
        payer,
        mint,
        associated_token_account,
        delegate,
        decimals: 9,
    }
}

pub async fn setup_empty_associated_token_account() -> SetupContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    let (mint, associated_token_accounts) = setup_mint_with_tokens(
        &mut rpc,
        &payer,
        payer.pubkey(),
        Some(payer.pubkey()),
        9,
        vec![(0, payer.pubkey())],
    )
    .await;

    let delegate = Keypair::new();

    SetupContext {
        rpc,
        payer,
        mint,
        associated_token_account: associated_token_accounts[0],
        delegate,
        decimals: 9,
    }
}

pub async fn setup_frozen() -> SetupContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let initial_amount = 1_000_000u64;

    let (mint, associated_token_accounts) = setup_mint_with_tokens(
        &mut rpc,
        &payer,
        payer.pubkey(),
        Some(payer.pubkey()),
        9,
        vec![(initial_amount, payer.pubkey())],
    )
    .await;

    let associated_token_account = associated_token_accounts[0];
    let delegate = Keypair::new();

    let approve_instruction = Approve {
        token_account: associated_token_account,
        delegate: delegate.pubkey(),
        owner: payer.pubkey(),
        amount: initial_amount,
        fee_payer: payer.pubkey(),
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[approve_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let freeze_instruction = Freeze {
        token_account: associated_token_account,
        mint,
        freeze_authority: payer.pubkey(),
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[freeze_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    SetupContext {
        rpc,
        payer,
        mint,
        associated_token_account,
        delegate,
        decimals: 9,
    }
}

pub struct SplMintContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint: Pubkey,
}

pub async fn setup_spl_mint_context() -> SplMintContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint = setup_spl_mint(&mut rpc, &payer, 9).await;

    SplMintContext { rpc, payer, mint }
}

/// Returns initialized test RPC and payer keypair.
///
/// Uses `with_prover=true` to spawn prover server for `get_validity_proof()` calls.
/// Action examples (create_mint, mint_to) need proofs to create compressed addresses.
// We must create a compressed address at creation to ensure the mint does not exist yet
pub async fn setup_rpc_and_payer() -> (LightProgramTest, Keypair) {
    let rpc = LightProgramTest::new(ProgramTestConfig::new(true, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    (rpc, payer)
}

pub struct WrapContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint: Pubkey,
    pub source_associated_token_account: Pubkey,
    pub light_associated_token_account: Pubkey,
    pub decimals: u8,
}

/// Sets up SPL mint with interface PDA, SPL associated token account with tokens, and empty Light associated token account.
pub async fn setup_for_wrap() -> WrapContext {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let decimals = 9u8;

    let mint = setup_spl_mint(&mut rpc, &payer, decimals).await;
    let source_associated_token_account = setup_spl_associated_token_account(&mut rpc, &payer, &mint, &payer.pubkey(), 1_000_000).await;

    let light_associated_token_account = derive_token_ata(&payer.pubkey(), &mint);
    let create_associated_token_account_instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), payer.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[create_associated_token_account_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    WrapContext {
        rpc,
        payer,
        mint,
        source_associated_token_account,
        light_associated_token_account,
        decimals,
    }
}

pub struct UnwrapContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint: Pubkey,
    pub destination_associated_token_account: Pubkey,
    pub light_associated_token_account: Pubkey,
    pub decimals: u8,
}

/// Sets up SPL mint with interface PDA, empty SPL associated token account, and Light associated token account with tokens.
pub async fn setup_for_unwrap() -> UnwrapContext {
    use light_token_client::actions::Wrap;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let decimals = 9u8;

    // Create SPL mint and SPL associated token account with tokens
    let mint = setup_spl_mint(&mut rpc, &payer, decimals).await;
    let spl_associated_token_account = setup_spl_associated_token_account(&mut rpc, &payer, &mint, &payer.pubkey(), 1_000_000).await;

    // Create empty Light associated token account
    let light_associated_token_account = derive_token_ata(&payer.pubkey(), &mint);
    let create_associated_token_account_instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), payer.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[create_associated_token_account_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Move tokens from SPL associated token account to Light associated token account
    Wrap {
        source_spl_ata: spl_associated_token_account,
        destination: light_associated_token_account,
        mint,
        amount: 1_000_000,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // After wrap, spl_associated_token_account is empty and serves as unwrap destination
    UnwrapContext {
        rpc,
        payer,
        mint,
        destination_associated_token_account: spl_associated_token_account,
        light_associated_token_account,
        decimals,
    }
}
