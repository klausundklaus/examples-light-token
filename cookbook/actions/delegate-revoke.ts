import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc, bn } from "@lightprotocol/stateless.js";
import {
    createMint,
    mintTo,
    approve,
    revoke,
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

    // Setup: Get compressed tokens
    const { mint } = await createMint(rpc, payer, payer.publicKey, 9);
    await mintTo(rpc, payer, mint, payer.publicKey, payer, bn(1000));

    // Approve then revoke delegation
    const delegate = Keypair.generate();
    await approve(rpc, payer, mint, bn(500), payer, delegate.publicKey);

    const delegatedAccounts = await rpc.getCompressedTokenAccountsByDelegate(
        delegate.publicKey,
        { mint }
    );
    const tx = await revoke(rpc, payer, delegatedAccounts.items, payer);

    console.log("Tx:", tx);
})();
