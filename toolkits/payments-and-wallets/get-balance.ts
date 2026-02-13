import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    transferInterface,
    getAtaInterface,
} from "@lightprotocol/compressed-token";
import { wrap } from "@lightprotocol/compressed-token/unified";
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
    // Setup: Create SPL mint, fund sender
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

    const splAta = await createAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        payer.publicKey,
        undefined,
        TOKEN_PROGRAM_ID
    );
    await mintTo(rpc, payer, mint, splAta, payer, 1000);

    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const senderAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await wrap(rpc, payer, splAta, senderAta, payer, mint, BigInt(1000));

    // Transfer to recipient
    const recipient = Keypair.generate();
    await createAtaInterface(rpc, payer, mint, recipient.publicKey);
    await transferInterface(
        rpc,
        payer,
        senderAta,
        mint,
        recipient.publicKey,
        payer,
        100
    );

    // Get recipient's balance
    const recipientAta = getAssociatedTokenAddressInterface(
        mint,
        recipient.publicKey
    );
    const { parsed: account } = await getAtaInterface(
        rpc,
        recipientAta,
        recipient.publicKey,
        mint
    );
    console.log("Recipient's balance:", account.amount);
})();
