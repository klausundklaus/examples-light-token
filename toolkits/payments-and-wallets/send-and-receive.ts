import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    transferInterface,
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
    // 1. Create SPL mint (includes SPL interface PDA registration)
    const { mint } = await createMintInterface(
        rpc, payer, payer, null, 9,
        undefined, undefined, TOKEN_PROGRAM_ID,
    );

    // 2. Fund payer with SPL tokens
    const splAta = await createAssociatedTokenAccount(
        rpc, payer, mint, payer.publicKey, undefined, TOKEN_PROGRAM_ID,
    );
    await mintTo(rpc, payer, mint, splAta, payer, 1_000_000);

    // 3. Create c-token ATA and wrap SPL tokens into it
    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const senderAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await wrap(rpc, payer, splAta, senderAta, payer, mint, BigInt(1_000_000));

    // 4. Transfer from payer to recipient
    const recipient = Keypair.generate();
    const txId = await transferInterface(
        rpc,
        payer,
        senderAta,
        mint,
        recipient.publicKey,
        payer,
        100,
    );

    console.log("Tx:", txId);
})();
