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
import * as multisig from "@sqds/multisig";
import { homedir } from "os";
import { readFileSync } from "fs";

const { Permissions } = multisig.types;

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
    // 1. Create SPL mint (includes SPL interface PDA registration)
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

    // 2. Create SPL ATA, mint tokens, wrap into light-token ATA
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
    const lightTokenAta = getAssociatedTokenAddressInterface(
        mint,
        payer.publicKey
    );
    await wrap(rpc, payer, splAta, lightTokenAta, payer, mint, BigInt(1_000_000));

    // 3. Create a 1-of-1 Squads multisig
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

    // 4. Create a light-token ATA owned by the vault (off-curve)
    await createAtaInterface(rpc, payer, mint, vaultPda, true);

    // 5. Transfer light tokens to the vault
    const sig = await transferInterface(
        rpc,
        payer,
        lightTokenAta,
        mint,
        vaultPda,
        payer,
        500_000
    );

    console.log("Vault:", vaultPda.toBase58());
    console.log("Tx:", sig);
})();
