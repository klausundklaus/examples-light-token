import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc, bn } from "@lightprotocol/stateless.js";
import {
    getOrCreateAtaInterface,
    transferInterface,
    createMintInterface,
    mintToInterface,
    getAtaInterface,
} from "@lightprotocol/compressed-token/unified";
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
    // 1. Create mint
    const { mint } = await createMintInterface(rpc, payer, payer, null, 9);

    // 2. Create ATA for payer (source)
    const { parsed: sourceAta } = await getOrCreateAtaInterface(
        rpc,
        payer,
        mint,
        payer
    );

    // 3. Mint to payer's ATA
    await mintToInterface(rpc, payer, mint, sourceAta.address, payer, bn(1000));

    // 4. Create ATA for recipient
    const recipient = Keypair.generate();
    const { parsed: recipientAta } = await getOrCreateAtaInterface(
        rpc,
        payer,
        mint,
        recipient
    );

    // 5. Transfer from payer to recipient
    await transferInterface(
        rpc,
        payer,
        sourceAta.address,
        mint,
        recipient.publicKey,
        payer,
        bn(100)
    );

    // 6. Get recipient's balance after transfer
    const { parsed: account } = await getAtaInterface(
        rpc,
        recipientAta.address,
        recipient.publicKey,
        mint
    );
    console.log("Recipient's balance:", account.amount);
})();
