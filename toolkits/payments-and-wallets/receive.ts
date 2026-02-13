import "dotenv/config";
import {
    Keypair,
    Transaction,
    sendAndConfirmTransaction,
} from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    transferInterface,
    createLoadAtaInstructions,
} from "@lightprotocol/compressed-token";
import { wrap } from "@lightprotocol/compressed-token/unified";
import {
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccount,
    mintTo,
} from "@solana/spl-token";
import { homedir } from "os";
import { readFileSync } from "fs";

const rpc = createRpc();

const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

(async function () {
    // --- Setup: create SPL mint, fund sender, transfer to recipient ---
    const { mint } = await createMintInterface(
        rpc, payer, payer, null, 9,
        undefined, undefined, TOKEN_PROGRAM_ID,
    );

    const splAta = await createAssociatedTokenAccount(
        rpc, payer, mint, payer.publicKey, undefined, TOKEN_PROGRAM_ID,
    );
    await mintTo(rpc, payer, mint, splAta, payer, 1_000_000);

    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const senderAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await wrap(rpc, payer, splAta, senderAta, payer, mint, BigInt(1_000_000));

    // Transfer to a fresh recipient so they have cold tokens
    const recipient = Keypair.generate();
    await transferInterface(rpc, payer, senderAta, mint, recipient.publicKey, payer, 500);

    // --- Receive: load creates ATA if needed + pulls cold state to hot ---
    const recipientAta = getAssociatedTokenAddressInterface(mint, recipient.publicKey);

    // Returns TransactionInstruction[][]. Each inner array is one txn.
    // Almost always one. Empty = noop.
    const instructions = await createLoadAtaInstructions(
        rpc,
        recipientAta,
        recipient.publicKey,
        mint,
        payer.publicKey,
    );

    for (const ixs of instructions) {
        const tx = new Transaction().add(...ixs);
        await sendAndConfirmTransaction(rpc, tx, [payer]);
    }

    console.log("Recipient ATA:", recipientAta.toBase58());
})();
