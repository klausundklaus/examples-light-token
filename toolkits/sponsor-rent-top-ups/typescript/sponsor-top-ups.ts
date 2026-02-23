import "dotenv/config";
import { Keypair, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import {
    createAtaInterface,
    createLightTokenTransferInstruction,
    getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";
import { homedir } from "os";
import { readFileSync } from "fs";
import { setup } from "./setup.js";

// devnet:
// const RPC_URL = `https://devnet.helius-rpc.com?api-key=${process.env.API_KEY!}`;
// const rpc = createRpc(RPC_URL);
// localnet:
const rpc = createRpc();

// Top-Up Sponsor: your application server, pays SOL for rent top-ups
const sponsor = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8")),
    ),
);

// User: only signs to authorize the transfer
const sender = Keypair.generate();

(async function () {
    const { mint, senderAta } = await setup(rpc, sponsor, sender);

    // Create recipient associated token account
    const recipient = Keypair.generate();
    await createAtaInterface(rpc, sponsor, mint, recipient.publicKey);
    const recipientAta = getAssociatedTokenAddressInterface(mint, recipient.publicKey);

    const ix = createLightTokenTransferInstruction(
        senderAta,
        recipientAta,
        sender.publicKey,
        500_000,
        sponsor.publicKey,
    );

    const tx = new Transaction().add(ix);
    const sig = await sendAndConfirmTransaction(rpc, tx, [sponsor, sender]);

    console.log("Tx:", sig);
})();
