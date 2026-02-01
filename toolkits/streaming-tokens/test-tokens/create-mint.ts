import dotenv from "dotenv";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";
dotenv.config({ path: resolve(dirname(fileURLToPath(import.meta.url)), "../.env") });
import { Keypair } from "@solana/web3.js";
import { createRpc, confirmTx } from "@lightprotocol/stateless.js";
import { createMintInterface, createTokenMetadata } from "@lightprotocol/compressed-token";
import { homedir } from "os";
import { readFileSync } from "fs";

const RPC_URL = `https://devnet.helius-rpc.com?api-key=${process.env.API_KEY!}`;
const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

(async function () {
    const rpc = createRpc(RPC_URL);

    // Airdrop if needed
    const balance = await rpc.getBalance(payer.publicKey);
    if (balance < 1_000_000_000) {
        const sig = await rpc.requestAirdrop(payer.publicKey, 2_000_000_000);
        await confirmTx(rpc, sig);
    }

    const { mint, transactionSignature } = await createMintInterface(
        rpc,
        payer,
        payer,
        null,
        9,
        undefined,
        undefined,
        undefined,
        createTokenMetadata("StreamTest", "STR", "https://example.com/metadata.json")
    );

    console.log("Mint:", mint.toBase58());
    console.log("Tx:", transactionSignature);
})();
