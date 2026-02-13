import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { createRpc, bn } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    mintToInterface,
    getOrCreateAtaInterface,
    getAssociatedTokenAddressInterface,
    transferInterface,
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
    const sourceAtaAddress = getAssociatedTokenAddressInterface(
        mint,
        payer.publicKey
    );

    // Loads cold balances, creates recipient ATA, transfers.
    const sig = await transferInterface(
        rpc,
        payer,
        sourceAtaAddress,
        mint,
        recipient.publicKey,
        payer,
        bn(100)
    );

    console.log("Tx:", sig);
})();
