import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc, bn } from "@lightprotocol/stateless.js";
import {
    createMint,
    mintTo,
    mergeTokenAccounts,
} from "@lightprotocol/compressed-token";
import { homedir } from "os";
import { readFileSync } from "fs";

// devnet:
const RPC_URL = `https://devnet.helius-rpc.com?api-key=${process.env.API_KEY!}`;
// localnet:
// const RPC_URL = undefined;
const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

(async function () {
    // devnet:
    const rpc = createRpc(RPC_URL);
    // localnet:
    // const rpc = createRpc();

    // Setup: Create multiple compressed token accounts
    const { mint } = await createMint(rpc, payer, payer.publicKey, 9);
    for (let i = 0; i < 5; i++) {
        await mintTo(rpc, payer, mint, payer.publicKey, payer, bn(100));
    }

    // Merge multiple accounts into one
    const tx = await mergeTokenAccounts(rpc, payer, mint, payer);

    console.log("Tx:", tx);
})();
