import "dotenv/config";
import {
    Keypair,
    Transaction,
    sendAndConfirmTransaction,
} from "@solana/web3.js";
import { createRpc, bn } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    mintToInterface,
    getOrCreateAtaInterface,
    createTransferInterfaceInstructions,
} from "@lightprotocol/compressed-token/unified";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
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
    // Setup: Create SPL mint with interface, fund an ATA
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
    const { parsed: sourceAta } = await getOrCreateAtaInterface(
        rpc,
        payer,
        mint,
        payer
    );
    await mintToInterface(
        rpc,
        payer,
        mint,
        sourceAta.address,
        payer,
        bn(1_000_000)
    );

    const recipient = Keypair.generate();

    // Returns TransactionInstruction[][]. Each inner array is one txn.
    // Almost always this returns one atomic transaction.
    const instructions = await createTransferInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        bn(100),
        payer.publicKey,
        recipient.publicKey
    );

    for (const ixs of instructions) {
        const tx = new Transaction().add(...ixs);
        const sig = await sendAndConfirmTransaction(rpc, tx, [payer]);
        console.log("Tx:", sig);
    }
})();
