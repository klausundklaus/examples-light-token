import "dotenv/config";
import { Keypair, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import { createTransferInterfaceInstructions } from "@lightprotocol/compressed-token/unified";
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
    const { mint } = await setup(rpc, sponsor, sender);

    const recipient = Keypair.generate();

    // Loads cold balances, creates recipient ATA if needed.
    // Returns TransactionInstruction[][] — each inner array is one transaction.
    const instructions = await createTransferInterfaceInstructions(
        rpc,
        sponsor.publicKey,    // payer: sponsor covers rent top-ups
        mint,
        500_000,
        sender.publicKey,     // authority: user signs to authorize transfer
        recipient.publicKey,
    );

    for (const ixs of instructions) {
        const tx = new Transaction().add(...ixs);
        const sig = await sendAndConfirmTransaction(rpc, tx, [sponsor, sender]);
        console.log("Tx:", sig);
    }
})();
