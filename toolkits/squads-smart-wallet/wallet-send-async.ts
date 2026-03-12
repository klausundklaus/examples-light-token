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

const RPC_URL = process.env.RPC_URL || "http://127.0.0.1:8899";
const rpc = createRpc(RPC_URL);

const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

async function confirmTx(sig: string) {
    await rpc.confirmTransaction(sig, "confirmed");
}

(async function () {
    // 1. Setup: Create mint, fund payer, create smart account, fund wallet
    const { mint } = await createMintInterface(rpc, payer, payer, null, 9);
    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const payerAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await mintToInterface(rpc, payer, mint, payerAta, payer, 1_000_000);

    const programConfig =
        await smartAccount.accounts.ProgramConfig.fromAccountAddress(
            rpc,
            smartAccount.getProgramConfigPda({})[0]
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
            {
                key: payer.publicKey,
                permissions: smartAccount.types.Permissions.all(),
            },
        ],
        timeLock: 0,
        rentCollector: null,
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(createSig);

    // Fund wallet with LTs
    await createAtaInterface(rpc, payer, mint, walletPda, true);
    const walletAta = getAssociatedTokenAddressInterface(mint, walletPda, true);
    const fundIx = createLightTokenTransferInstruction(
        payerAta,
        walletAta,
        payer.publicKey,
        500_000
    );
    const { blockhash: bh1 } = await rpc.getLatestBlockhash();
    await sendAndConfirmTx(rpc, buildAndSignTx([fundIx], payer, bh1, []));

    // Fund wallet with SOL
    const { blockhash: bh2 } = await rpc.getLatestBlockhash();
    const solMsg = new TransactionMessage({
        payerKey: payer.publicKey,
        recentBlockhash: bh2,
        instructions: [
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: walletPda,
                lamports: 10_000_000,
            }),
        ],
    }).compileToV0Message();
    const solTx = new VersionedTransaction(solMsg);
    solTx.sign([payer]);
    await confirmTx(await rpc.sendRawTransaction(solTx.serialize()));

    // 2. Smart wallet sends LTs via async proposal flow
    const recipient = Keypair.generate();
    await createAtaInterface(rpc, payer, mint, recipient.publicKey);
    const recipientAta = getAssociatedTokenAddressInterface(
        mint,
        recipient.publicKey
    );

    const transferIx = createLightTokenTransferInstruction(
        walletAta,
        recipientAta,
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

    // Create transaction
    const { blockhash: bh3 } = await rpc.getLatestBlockhash();
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
            recentBlockhash: bh3,
            instructions: [transferIx],
        }),
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(createTxSig);
    console.log("Transaction created");

    // Create proposal
    const proposalSig = await smartAccount.rpc.createProposal({
        connection: rpc,
        feePayer: payer,
        settingsPda,
        transactionIndex: txIndex,
        creator: payer,
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(proposalSig);
    console.log("Proposal created");

    // Approve proposal
    const approveSig = await smartAccount.rpc.approveProposal({
        connection: rpc,
        feePayer: payer,
        settingsPda,
        transactionIndex: txIndex,
        signer: payer,
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(approveSig);
    console.log("Proposal approved");

    // Execute
    const execSig = await smartAccount.rpc.executeTransaction({
        connection: rpc,
        feePayer: payer,
        settingsPda,
        transactionIndex: txIndex,
        signer: payer.publicKey,
        signers: [payer],
        sendOptions: { skipPreflight: true },
    });
    await confirmTx(execSig);

    console.log("Wallet:", walletPda.toBase58());
    console.log("Recipient:", recipient.publicKey.toBase58());
    console.log("Async transfer tx:", execSig);
})();
