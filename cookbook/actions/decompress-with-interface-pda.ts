import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc, bn } from "@lightprotocol/stateless.js";
import {
    createMint,
    mintTo,
    decompress,
    getTokenPoolInfos,
    selectTokenPoolInfosForDecompression,
    selectSplInterfaceInfosForDecompression,
    getSplInterfaceInfos,
} from "@lightprotocol/compressed-token";
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

async function main() {
    // devnet:
    const rpc = createRpc(RPC_URL);
    // localnet:
    // const rpc = createRpc();

    const { mint } = await createMint(rpc, payer, payer.publicKey, 9);

    await mintTo(rpc, payer, mint, payer.publicKey, payer, bn(1000));

    const splAta = await createAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        payer.publicKey
    );

    // Get and select Interface Info for decompression
    const amount = bn(500);
    const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
    const splInterfaceInfo = selectSplInterfaceInfosForDecompression(
        splInterfaceInfos,
        amount
    );

    const signature = await decompress(
        rpc,
        payer,
        mint,
        amount,
        payer,
        splAta,
        splInterfaceInfo
    );

    console.log(`Decompressed ${amount.toString()} tokens`);
    console.log("Tx:", signature);
}

main().catch(console.error);
