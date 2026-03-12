import "dotenv/config";
import {
    Keypair,
    PublicKey,
    SystemProgram,
    TransactionMessage,
    VersionedTransaction,
} from "@solana/web3.js";
import {
    createRpc,
    buildAndSignTx,
    sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    mintToInterface,
    createLightTokenTransferInstruction,
} from "@lightprotocol/compressed-token";
import * as smartAccount from "@sqds/smart-account";
import { homedir } from "os";
import { readFileSync } from "fs";

const { Permission, Permissions } = smartAccount.types;

const RPC_URL = process.env.RPC_URL || "http://127.0.0.1:8899";
const rpc = createRpc(RPC_URL);

const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

function assert(condition: boolean, message: string) {
    if (!condition) throw new Error(`FAIL: ${message}`);
    console.log(`PASS: ${message}`);
}

async function accountExists(pubkey: PublicKey): Promise<boolean> {
    const info = await rpc.getAccountInfo(pubkey);
    return info !== null && info.value !== null;
}

async function confirmTx(sig: string) {
    await rpc.confirmTransaction(sig, "confirmed");
}

(async function () {
    console.log("=== Squads Smart Account + Light Token Integration Test ===\n");

    // ── Setup: Create Light Token mint and fund payer ──
    const { mint } = await createMintInterface(rpc, payer, payer, null, 9);
    console.log("Mint:", mint.toBase58());

    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const payerAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await mintToInterface(rpc, payer, mint, payerAta, payer, 1_000_000);
    console.log("Payer ATA funded with 1,000,000 tokens\n");

    // ── Step 1: Create Smart Account (1-of-1, timeLock=0) ──
    console.log("--- Step 1: Create smart account ---");

    const programConfigPda = smartAccount.getProgramConfigPda({})[0];
    const programConfig =
        await smartAccount.accounts.ProgramConfig.fromAccountAddress(
            rpc,
            programConfigPda
        );
    const accountIndex =
        BigInt(programConfig.smartAccountIndex.toString()) + 1n;

    const [settingsPda] = smartAccount.getSettingsPda({ accountIndex });
    const [walletPda] = smartAccount.getSmartAccountPda({
        settingsPda,
        accountIndex: 0,
    });

    const createSig = await smartAccount.rpc.createSmartAccount({
        connection: rpc,
        treasury: programConfig.treasury,
        creator: payer,
        settings: settingsPda,
        settingsAuthority: null,
        threshold: 1,
        signers: [
            { key: payer.publicKey, permissions: Permissions.all() },
        ],
        timeLock: 0,
        rentCollector: null,
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(createSig);

    console.log("Settings PDA:", settingsPda.toBase58());
    console.log("Wallet PDA:", walletPda.toBase58());
    assert(await accountExists(settingsPda), "Smart account created");

    // ── Step 2: Fund smart wallet with Light Tokens ──
    console.log("\n--- Step 2: Fund smart wallet with Light Tokens ---");

    // Create Light Token ATA for the wallet PDA (off-curve, so allowOwnerOffCurve=true)
    await createAtaInterface(rpc, payer, mint, walletPda, true);
    const walletAta = getAssociatedTokenAddressInterface(mint, walletPda, true);
    console.log("Wallet ATA:", walletAta.toBase58());

    // Transfer LTs from payer to wallet (no approval needed — anyone can fund)
    const fundIx = createLightTokenTransferInstruction(
        payerAta,
        walletAta,
        payer.publicKey,
        500_000
    );
    const { blockhash: bh1 } = await rpc.getLatestBlockhash();
    const fundTx = buildAndSignTx([fundIx], payer, bh1, []);
    const fundSig = await sendAndConfirmTx(rpc, fundTx);
    console.log("Fund tx:", fundSig);
    assert(await accountExists(walletAta), "Wallet ATA exists on-chain");

    // Fund wallet with SOL for inner transaction fees
    const solFundIx = SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: walletPda,
        lamports: 10_000_000,
    });
    const { blockhash: bh2 } = await rpc.getLatestBlockhash();
    const solFundMsg = new TransactionMessage({
        payerKey: payer.publicKey,
        recentBlockhash: bh2,
        instructions: [solFundIx],
    }).compileToV0Message();
    const solFundTx = new VersionedTransaction(solFundMsg);
    solFundTx.sign([payer]);
    const solFundSig = await rpc.sendRawTransaction(solFundTx.serialize());
    await confirmTx(solFundSig);
    console.log("Wallet funded with 0.01 SOL");

    // ── Step 3: Smart wallet sends LTs — sync execution ──
    console.log("\n--- Step 3: Smart wallet sends LTs (sync) ---");

    const recipientA = Keypair.generate();
    await createAtaInterface(rpc, payer, mint, recipientA.publicKey);
    const recipientAtaA = getAssociatedTokenAddressInterface(
        mint,
        recipientA.publicKey
    );

    // Build Light Token transfer instruction (wallet → recipient A)
    const transferSyncIx = createLightTokenTransferInstruction(
        walletAta,
        recipientAtaA,
        walletPda,     // owner of source ATA
        100_000,
        walletPda      // fee payer = wallet PDA
    );

    // Compile for synchronous execution
    const { instructions: syncInstructions, accounts: syncAccounts } =
        smartAccount.utils.instructionsToSynchronousTransactionDetails({
            vaultPda: walletPda,
            members: [payer.publicKey],
            transaction_instructions: [transferSyncIx],
        });

    // Build the sync execution instruction
    const syncExecIx = smartAccount.instructions.executeTransactionSync({
        settingsPda,
        numSigners: 1,
        accountIndex: 0,
        instructions: syncInstructions,
        instruction_accounts: syncAccounts,
    });

    // Send as a single transaction — immediate execution, no proposal needed
    const { blockhash: bh3 } = await rpc.getLatestBlockhash();
    const syncMsg = new TransactionMessage({
        payerKey: payer.publicKey,
        recentBlockhash: bh3,
        instructions: [syncExecIx],
    }).compileToV0Message();
    const syncTx = new VersionedTransaction(syncMsg);
    syncTx.sign([payer]);
    const syncSig = await rpc.sendRawTransaction(syncTx.serialize(), {
        skipPreflight: true,
    });
    await confirmTx(syncSig);
    console.log("Sync transfer tx:", syncSig);
    assert(
        await accountExists(recipientAtaA),
        "Recipient A ATA exists (sync transfer)"
    );

    // ── Step 4: Smart wallet sends LTs — async proposal flow ──
    console.log("\n--- Step 4: Smart wallet sends LTs (async) ---");

    const recipientB = Keypair.generate();
    await createAtaInterface(rpc, payer, mint, recipientB.publicKey);
    const recipientAtaB = getAssociatedTokenAddressInterface(
        mint,
        recipientB.publicKey
    );

    // Build Light Token transfer instruction (wallet → recipient B)
    const transferAsyncIx = createLightTokenTransferInstruction(
        walletAta,
        recipientAtaB,
        walletPda,
        100_000,
        walletPda
    );

    // Read current transaction index
    const settings = await smartAccount.accounts.Settings.fromAccountAddress(
        rpc,
        settingsPda
    );
    const txIndex = BigInt(settings.transactionIndex.toString()) + 1n;

    // 4a. Create transaction
    const { blockhash: bh4 } = await rpc.getLatestBlockhash();
    const createTxSig = await smartAccount.rpc.createTransaction({
        connection: rpc,
        feePayer: payer,
        settingsPda,
        transactionIndex: txIndex,
        creator: payer.publicKey,
        accountIndex: 0,
        ephemeralSigners: 0,
        transactionMessage: new TransactionMessage({
            payerKey: walletPda,
            recentBlockhash: bh4,
            instructions: [transferAsyncIx],
        }),
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(createTxSig);
    console.log("Transaction created:", createTxSig);

    // 4b. Create proposal
    const proposalSig = await smartAccount.rpc.createProposal({
        connection: rpc,
        feePayer: payer,
        settingsPda,
        transactionIndex: txIndex,
        creator: payer,
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(proposalSig);
    console.log("Proposal created:", proposalSig);

    // 4c. Approve proposal
    const approveSig = await smartAccount.rpc.approveProposal({
        connection: rpc,
        feePayer: payer,
        settingsPda,
        transactionIndex: txIndex,
        signer: payer,
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(approveSig);
    console.log("Proposal approved:", approveSig);

    // 4d. Execute transaction
    const executeSig = await smartAccount.rpc.executeTransaction({
        connection: rpc,
        feePayer: payer,
        settingsPda,
        transactionIndex: txIndex,
        signer: payer.publicKey,
        signers: [payer],
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(executeSig);
    console.log("Transaction executed:", executeSig);

    assert(
        await accountExists(recipientAtaB),
        "Recipient B ATA exists (async transfer)"
    );

    // ── Step 5: Verify ──
    console.log("\n--- Step 5: Verify ---");
    assert(await accountExists(walletAta), "Wallet ATA still exists");
    assert(
        await accountExists(recipientAtaA),
        "Recipient A received tokens (sync)"
    );
    assert(
        await accountExists(recipientAtaB),
        "Recipient B received tokens (async)"
    );

    console.log("\n=== All tests passed ===");
})();
