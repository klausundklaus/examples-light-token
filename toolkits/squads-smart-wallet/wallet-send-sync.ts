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
    await rpc.confirmTransaction(createSig, "confirmed");

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
    const solSig = await rpc.sendRawTransaction(solTx.serialize());
    await rpc.confirmTransaction(solSig, "confirmed");

    // 2. Smart wallet sends LTs via sync execution
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

    // Compile for sync execution
    const { instructions, accounts } =
        smartAccount.utils.instructionsToSynchronousTransactionDetails({
            vaultPda: walletPda,
            members: [payer.publicKey],
            transaction_instructions: [transferIx],
        });

    const syncIx = smartAccount.instructions.executeTransactionSync({
        settingsPda,
        numSigners: 1,
        accountIndex: 0,
        instructions,
        instruction_accounts: accounts,
    });

    // Execute in a single transaction — no proposal needed
    const { blockhash: bh3 } = await rpc.getLatestBlockhash();
    const msg = new TransactionMessage({
        payerKey: payer.publicKey,
        recentBlockhash: bh3,
        instructions: [syncIx],
    }).compileToV0Message();
    const tx = new VersionedTransaction(msg);
    tx.sign([payer]);
    const sig = await rpc.sendRawTransaction(tx.serialize(), {
        skipPreflight: true,
    });
    await rpc.confirmTransaction(sig, "confirmed");

    console.log("Wallet:", walletPda.toBase58());
    console.log("Recipient:", recipient.publicKey.toBase58());
    console.log("Sync transfer tx:", sig);
})();
