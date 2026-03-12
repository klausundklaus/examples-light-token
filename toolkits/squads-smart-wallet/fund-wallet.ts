import "dotenv/config";
import {
    Keypair,
    PublicKey,
    SystemProgram,
    TransactionMessage,
    VersionedTransaction,
} from "@solana/web3.js";
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
import * as smartAccount from "@sqds/smart-account";
import { homedir } from "os";
import { readFileSync } from "fs";

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
    const payerAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await mintToInterface(rpc, payer, mint, payerAta, payer, 1_000_000);

    // 2. Create a 1-of-1 smart account
    const programConfig =
        await smartAccount.accounts.ProgramConfig.fromAccountAddress(
            rpc,
            smartAccount.getProgramConfigPda({})[0]
        );
    const accountIndex =
        BigInt(programConfig.smartAccountIndex.toString()) + 1n;
    const [settingsPda] = smartAccount.getSettingsPda({ accountIndex });
    const [walletPda] = smartAccount.getSmartAccountPda({
        settingsPda,
        accountIndex: 0,
    });

    const createSig = await smartAccount.rpc.createSmartAccount({
        connection: rpc,
        treasury: programConfig.treasury,
        creator: payer,
        settings: settingsPda,
        settingsAuthority: null,
        threshold: 1,
        signers: [
            {
                key: payer.publicKey,
                permissions: smartAccount.types.Permissions.all(),
            },
        ],
        timeLock: 0,
        rentCollector: null,
        sendOptions: { skipPreflight: true },
    });
    await rpc.confirmTransaction(createSig, "confirmed");

    // 3. Create Light Token ATA for the wallet (off-curve PDA)
    await createAtaInterface(rpc, payer, mint, walletPda, true);
    const walletAta = getAssociatedTokenAddressInterface(mint, walletPda, true);

    // 4. Transfer Light Tokens to the wallet — no approval needed
    const ix = createLightTokenTransferInstruction(
        payerAta,
        walletAta,
        payer.publicKey,
        500_000
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx([ix], payer, blockhash, []);
    const sig = await sendAndConfirmTx(rpc, tx);

    console.log("Settings PDA:", settingsPda.toBase58());
    console.log("Wallet PDA:", walletPda.toBase58());
    console.log("Wallet ATA:", walletAta.toBase58());
    console.log("Fund tx:", sig);
})();
