import "dotenv/config";
import { Keypair, TransactionMessage } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    getAtaInterface,
} from "@lightprotocol/compressed-token";
import {
    createTransferInterfaceInstructions,
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

const rpc = createRpc();

const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"))
    )
);

function assert(condition: boolean, message: string) {
    if (!condition) throw new Error(`FAIL: ${message}`);
    console.log(`PASS: ${message}`);
}

(async function () {
    console.log("=== Squads + Light Token Integration Test ===\n");

    // Setup: Create mint, fund payer
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
    const payerLightAta = getAssociatedTokenAddressInterface(
        mint,
        payer.publicKey
    );
    await wrap(rpc, payer, splAta, payerLightAta, payer, mint, BigInt(1_000_000));

    // Step 1: Create multisig + vault
    console.log("--- Step 1: Create Squads multisig ---");
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
    console.log("Multisig:", multisigPda.toBase58());
    console.log("Vault:", vaultPda.toBase58());
    assert(true, "Multisig + vault created");

    // Step 2: Transfer light tokens TO vault
    console.log("\n--- Step 2: Transfer light tokens to vault ---");
    await createAtaInterface(rpc, payer, mint, vaultPda, true);
    await transferInterface(
        rpc,
        payer,
        payerLightAta,
        mint,
        vaultPda,
        payer,
        500_000
    );

    const vaultLightAta = getAssociatedTokenAddressInterface(
        mint,
        vaultPda,
        true
    );
    const vaultAccount = await getAtaInterface(rpc, vaultLightAta, vaultPda, mint);
    assert(
        vaultAccount.parsed.amount.toString() === "500000",
        `Vault balance is 500000 (got ${vaultAccount.parsed.amount})`
    );

    // Step 3: Transfer light tokens FROM vault
    console.log("\n--- Step 3: Transfer light tokens from vault ---");
    const recipient = Keypair.generate();
    const transferIxBatches = await createTransferInterfaceInstructions(
        rpc,
        vaultPda,
        mint,
        100_000,
        vaultPda,
        recipient.publicKey
    );

    const multisigAccount = await multisig.accounts.Multisig.fromAccountAddress(
        rpc,
        multisigPda
    );
    let txIndex =
        BigInt(multisigAccount.transactionIndex.toString()) + BigInt(1);

    for (const ixs of transferIxBatches) {
        const transferMessage = new TransactionMessage({
            payerKey: vaultPda,
            recentBlockhash: (await rpc.getLatestBlockhash()).blockhash,
            instructions: ixs,
        });

        await multisig.rpc.vaultTransactionCreate({
            connection: rpc,
            feePayer: payer,
            multisigPda,
            transactionIndex: txIndex,
            creator: payer.publicKey,
            vaultIndex: 0,
            ephemeralSigners: 0,
            transactionMessage: transferMessage,
        });

        await multisig.rpc.proposalCreate({
            connection: rpc,
            feePayer: payer,
            multisigPda,
            transactionIndex: txIndex,
            creator: payer,
        });

        await multisig.rpc.proposalApprove({
            connection: rpc,
            feePayer: payer,
            multisigPda,
            transactionIndex: txIndex,
            member: payer,
        });

        await multisig.rpc.vaultTransactionExecute({
            connection: rpc,
            feePayer: payer,
            multisigPda,
            transactionIndex: txIndex,
            member: payer.publicKey,
            signers: [payer],
        });

        txIndex++;
    }

    // Step 4: Verify balances
    console.log("\n--- Step 4: Verify balances ---");
    const vaultAfter = await getAtaInterface(rpc, vaultLightAta, vaultPda, mint);
    assert(
        vaultAfter.parsed.amount.toString() === "400000",
        `Vault balance is 400000 (got ${vaultAfter.parsed.amount})`
    );

    const recipientLightAta = getAssociatedTokenAddressInterface(
        mint,
        recipient.publicKey
    );
    const recipientAccount = await getAtaInterface(
        rpc,
        recipientLightAta,
        recipient.publicKey,
        mint
    );
    assert(
        recipientAccount.parsed.amount.toString() === "100000",
        `Recipient balance is 100000 (got ${recipientAccount.parsed.amount})`
    );

    console.log("\n=== All tests passed ===");
})();
