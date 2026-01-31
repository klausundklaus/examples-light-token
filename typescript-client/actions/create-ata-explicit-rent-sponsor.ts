import "dotenv/config";
import { Keypair, PublicKey, ComputeBudgetProgram } from "@solana/web3.js";
import {
    createRpc,
    CTOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAssociatedTokenAccountInterfaceInstruction,
    getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";
import { homedir } from "os";
import { readFileSync } from "fs";

const LIGHT_TOKEN_CONFIG = new PublicKey(
    "ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg",
);
const LIGHT_TOKEN_RENT_SPONSOR = new PublicKey(
    "r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti",
);

const DEFAULT_COMPRESSIBLE_CONFIG = {
    tokenAccountVersion: 3,
    rentPayment: 16,
    compressionOnly: 1,
    writeTopUp: 766,
    compressToAccountPubkey: null,
};

// devnet:
// const RPC_URL = `https://devnet.helius-rpc.com?api-key=${process.env.API_KEY!}`;
// const rpc = createRpc(RPC_URL);
// localnet:
const rpc = createRpc();

const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8")),
    ),
);

(async function () {
    const { mint } = await createMintInterface(rpc, payer, payer, null, 9);
    console.log("Mint:", mint.toBase58());

    const owner = Keypair.generate();
    const associatedToken = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
    );

    const ix = createAssociatedTokenAccountInterfaceInstruction(
        payer.publicKey,
        associatedToken,
        owner.publicKey,
        mint,
        CTOKEN_PROGRAM_ID,
        CTOKEN_PROGRAM_ID,
        {
            compressibleConfig: DEFAULT_COMPRESSIBLE_CONFIG,
            configAccount: LIGHT_TOKEN_CONFIG,
            rentPayerPda: LIGHT_TOKEN_RENT_SPONSOR,
        },
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 50_000 }), ix],
        payer,
        blockhash,
        [],
    );
    const signature = await sendAndConfirmTx(rpc, tx);

    console.log("ATA:", associatedToken.toBase58());
    console.log("Tx:", signature);
})();
