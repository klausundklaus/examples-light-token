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
} from "@lightprotocol/compressed-token";
import {
    createTransferInterfaceInstructions,
    wrap,
} from "@lightprotocol/compressed-token/unified";
import {
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccount,
    mintTo,
} from "@solana/spl-token";
import { homedir } from "os";
import { readFileSync } from "fs";

// devnet:
// const RPC_URL = `https://devnet.helius-rpc.com?api-key=${process.env.API_KEY!}`;
// const rpc = createRpc(RPC_URL);
// localnet:
const rpc = createRpc();

const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

(async function () {
    // Setup: Create SPL mint (includes SPL interface PDA registration)
    const { mint } = await createMintInterface(
        rpc,
        payer,
        payer,
        null,
        9,
        undefined,
        undefined,
        TOKEN_PROGRAM_ID
    );

    // Fund payer with SPL tokens
    const splAta = await createAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        payer.publicKey,
        undefined,
        TOKEN_PROGRAM_ID
    );
    await mintTo(rpc, payer, mint, splAta, payer, 1_000_000);

    // Create light-token ATA and wrap SPL tokens into it
    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const cTokenAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await wrap(rpc, payer, splAta, cTokenAta, payer, mint, BigInt(1_000_000));

    const recipient = Keypair.generate();

    // Returns TransactionInstruction[][]. Each inner array is one txn.
    // Almost always this returns one atomic transaction.
    const instructions = await createTransferInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        100,
        payer.publicKey,
        recipient.publicKey
    );

    for (const ixs of instructions) {
        const tx = new Transaction().add(...ixs);
        const sig = await sendAndConfirmTransaction(rpc, tx, [payer]);
        console.log("Tx:", sig);
    }
})();
