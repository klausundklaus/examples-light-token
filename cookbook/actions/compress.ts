import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc, bn } from "@lightprotocol/stateless.js";
import { createMint, mintTo, decompress, compress } from "@lightprotocol/compressed-token";
import { createAssociatedTokenAccount } from "@solana/spl-token";
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

    // Setup: Get SPL tokens (needed to compress)
    const { mint } = await createMint(rpc, payer, payer.publicKey, 9);
    const splAta = await createAssociatedTokenAccount(rpc, payer, mint, payer.publicKey);
    await mintTo(rpc, payer, mint, payer.publicKey, payer, bn(1000));
    await decompress(rpc, payer, mint, bn(1000), payer, splAta);

    // Compress SPL tokens to cold storage
    const recipient = Keypair.generate();
    const tx = await compress(rpc, payer, mint, bn(500), payer, splAta, recipient.publicKey);

    console.log("Tx:", tx);
})();
