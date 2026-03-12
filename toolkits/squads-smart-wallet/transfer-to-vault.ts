import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import {
    createRpc,
    buildAndSignTx,
    sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    mintToInterface,
    createLightTokenTransferInstruction,
} from "@lightprotocol/compressed-token";
import * as multisig from "@sqds/multisig";
import { homedir } from "os";
import { readFileSync } from "fs";

const { Permissions } = multisig.types;

const RPC_URL = process.env.RPC_URL || "http://127.0.0.1:8899";
const rpc = createRpc(RPC_URL);

const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

(async function () {
    // 1. Create Light Token mint and mint tokens to payer
    const { mint } = await createMintInterface(rpc, payer, payer, null, 9);
    await createAtaInterface(rpc, payer, mint, payer.publicKey);
    const payerAta = getAssociatedTokenAddressInterface(
        mint,
        payer.publicKey
    );
    await mintToInterface(rpc, payer, mint, payerAta, payer, 1_000_000);

    // 2. Create a 1-of-1 Squads multisig
    const createKey = Keypair.generate();
    const [multisigPda] = multisig.getMultisigPda({
        createKey: createKey.publicKey,
    });
    const [vaultPda] = multisig.getVaultPda({ multisigPda, index: 0 });

    const programConfigPda = multisig.getProgramConfigPda({})[0];
    const programConfig =
        await multisig.accounts.ProgramConfig.fromAccountAddress(
            rpc,
            programConfigPda
        );

    await multisig.rpc.multisigCreateV2({
        connection: rpc,
        createKey,
        creator: payer,
        multisigPda,
        configAuthority: null,
        timeLock: 0,
        members: [
            { key: payer.publicKey, permissions: Permissions.all() },
        ],
        threshold: 1,
        rentCollector: null,
        treasury: programConfig.treasury,
    });

    // 3. Create a Light Token ATA owned by the vault (off-curve PDA)
    await createAtaInterface(rpc, payer, mint, vaultPda, true);
    const vaultAta = getAssociatedTokenAddressInterface(mint, vaultPda, true);

    // 4. Transfer Light Tokens to the vault
    //    Note: transferInterface() rejects off-curve recipients (PDA vaults).
    //    Use createLightTokenTransferInstruction which accepts any PublicKey.
    const transferIx = createLightTokenTransferInstruction(
        payerAta,
        vaultAta,
        payer.publicKey,
        500_000
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx([transferIx], payer, blockhash, []);
    const sig = await sendAndConfirmTx(rpc, tx);

    console.log("Vault:", vaultPda.toBase58());
    console.log("Vault ATA:", vaultAta.toBase58());
    console.log("Tx:", sig);
})();
