import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";
import {
    transferInterface,
    wrap,
} from "@lightprotocol/compressed-token/unified";
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
    // Setup: Create SPL mint (auto-registers SPL interface PDA)
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

    // Fund payer with SPL tokens, then wrap into light-token ATA
    const splAta = await createAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        payer.publicKey,
        undefined,
        TOKEN_PROGRAM_ID
    );
    await mintTo(rpc, payer, mint, splAta, payer, 1_000_000);
    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const cTokenAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await wrap(rpc, payer, splAta, cTokenAta, payer, mint, BigInt(1_000_000));

    const recipient = Keypair.generate();

    // Handles loading cold balances, creates recipient ATA, transfers.
    const sig = await transferInterface(
        rpc,
        payer,
        cTokenAta,
        mint,
        recipient.publicKey,
        payer,
        100
    );

    console.log("Tx:", sig);
})();
