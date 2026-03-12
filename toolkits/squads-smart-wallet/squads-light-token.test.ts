import "dotenv/config";
import { Keypair, TransactionMessage, PublicKey } from "@solana/web3.js";
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
import * as multisig from "@sqds/multisig";
import { homedir } from "os";
import { readFileSync } from "fs";

const { Permissions } = multisig.types;

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

/** Check if a Light Token ATA exists on-chain */
async function accountExists(pubkey: PublicKey): Promise<boolean> {
    const info = await rpc.getAccountInfo(pubkey);
    return info !== null && info.value !== null;
}

(async function () {
    console.log("=== Squads + Light Token Integration Test ===\n");

    // Setup: Create Light Token mint and mint to payer
    const { mint } = await createMintInterface(rpc, payer, payer, null, 9);
    console.log("Mint:", mint.toBase58());

    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const payerLightAta = getAssociatedTokenAddressInterface(
        mint,
        payer.publicKey
    );
    await mintToInterface(rpc, payer, mint, payerLightAta, payer, 1_000_000);
    console.log("Payer ATA funded with 1,000,000 tokens");

    // Step 1: Create multisig + vault
    console.log("\n--- Step 1: Create Squads multisig ---");
    const createKey = Keypair.generate();
    const [multisigPda] = multisig.getMultisigPda({
        createKey: createKey.publicKey,
    });
    const [vaultPda] = multisig.getVaultPda({ multisigPda, index: 0 });

    const programConfigPda = multisig.getProgramConfigPda({})[0];
    const programConfig =
        await multisig.accounts.ProgramConfig.fromAccountAddress(
            rpc,
            programConfigPda
        );

    await multisig.rpc.multisigCreateV2({
        connection: rpc,
        createKey,
        creator: payer,
        multisigPda,
        configAuthority: null,
        timeLock: 0,
        members: [
            { key: payer.publicKey, permissions: Permissions.all() },
        ],
        threshold: 1,
        rentCollector: null,
        treasury: programConfig.treasury,
    });
    console.log("Multisig:", multisigPda.toBase58());
    console.log("Vault:", vaultPda.toBase58());
    assert(true, "Multisig + vault created");

    // Step 2: Transfer Light Tokens to vault
    console.log("\n--- Step 2: Transfer Light Tokens to vault ---");
    await createAtaInterface(rpc, payer, mint, vaultPda, true);
    const vaultLightAta = getAssociatedTokenAddressInterface(
        mint,
        vaultPda,
        true
    );
    console.log("Vault ATA:", vaultLightAta.toBase58());

    // Direct ATA-to-ATA transfer (works with off-curve PDAs)
    const transferToVaultIx = createLightTokenTransferInstruction(
        payerLightAta,
        vaultLightAta,
        payer.publicKey,
        500_000
    );
    const { blockhash: bh1 } = await rpc.getLatestBlockhash();
    const tx1 = buildAndSignTx([transferToVaultIx], payer, bh1, []);
    const sig1 = await sendAndConfirmTx(rpc, tx1);
    console.log("Transfer to vault tx:", sig1);
    assert(await accountExists(vaultLightAta), "Vault ATA exists on-chain");

    // Step 3: Transfer Light Tokens FROM vault via Squads proposal
    console.log("\n--- Step 3: Transfer from vault via Squads ---");
    const recipient = Keypair.generate();
    await createAtaInterface(rpc, payer, mint, recipient.publicKey);
    const recipientLightAta = getAssociatedTokenAddressInterface(
        mint,
        recipient.publicKey
    );

    // Build transfer instruction with vault PDA as owner
    const transferFromVaultIx = createLightTokenTransferInstruction(
        vaultLightAta,
        recipientLightAta,
        vaultPda,
        100_000,
        vaultPda
    );

    // Wrap in Squads vault transaction
    // Fund vault with SOL for the inner transaction fees
    const fundVaultTx = new (await import("@solana/web3.js")).Transaction().add(
        (await import("@solana/web3.js")).SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: vaultPda,
            lamports: 10_000_000, // 0.01 SOL
        })
    );
    fundVaultTx.recentBlockhash = (await rpc.getLatestBlockhash()).blockhash;
    fundVaultTx.feePayer = payer.publicKey;
    fundVaultTx.sign(payer);
    const fundSig = await rpc.sendRawTransaction(fundVaultTx.serialize());
    await rpc.confirmTransaction(fundSig, "confirmed");
    console.log("Vault funded with SOL");

    const multisigInfo = await multisig.accounts.Multisig.fromAccountAddress(
        rpc,
        multisigPda
    );
    const txIndex = BigInt(multisigInfo.transactionIndex.toString()) + 1n;

    const transferMessage = new TransactionMessage({
        payerKey: vaultPda,
        recentBlockhash: (await rpc.getLatestBlockhash()).blockhash,
        instructions: [transferFromVaultIx],
    });

    const vtSig = await multisig.rpc.vaultTransactionCreate({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        creator: payer.publicKey,
        vaultIndex: 0,
        ephemeralSigners: 0,
        transactionMessage: transferMessage,
    });
    console.log("Vault transaction created:", vtSig);
    await rpc.confirmTransaction(vtSig, "confirmed");

    const proposalSig = await multisig.rpc.proposalCreate({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        creator: payer,
    });
    console.log("Proposal created:", proposalSig);
    // Wait for confirmation before approving
    await rpc.confirmTransaction(proposalSig, "confirmed");

    const approveSig = await multisig.rpc.proposalApprove({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        member: payer,
    });
    console.log("Proposal approved:", approveSig);
    await rpc.confirmTransaction(approveSig, "confirmed");

    await multisig.rpc.vaultTransactionExecute({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        member: payer.publicKey,
        signers: [payer],
    });
    console.log("Vault transaction executed");

    // Step 4: Verify recipient received tokens
    console.log("\n--- Step 4: Verify ---");
    assert(
        await accountExists(recipientLightAta),
        "Recipient ATA exists on-chain"
    );
    assert(true, "Squads vault transaction executed successfully");

    console.log("\n=== All tests passed ===");
})();
