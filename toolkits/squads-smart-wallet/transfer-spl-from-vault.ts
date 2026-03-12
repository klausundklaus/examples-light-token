import "dotenv/config";
import { Keypair, TransactionMessage, SystemProgram, Transaction } from "@solana/web3.js";
import {
    createRpc,
    buildAndSignTx,
    sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    createLightTokenTransferInstruction,
} from "@lightprotocol/compressed-token";
import { wrap, createUnwrapInstructions } from "@lightprotocol/compressed-token/unified";
import {
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccount,
    createTransferInstruction,
    getAssociatedTokenAddress,
    mintTo,
} from "@solana/spl-token";
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

    // 2. Mint SPL tokens, wrap into Light Token ATA
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

    // 4. Fund vault with Light Tokens using createLightTokenTransferInstruction
    //    (transferInterface rejects off-curve PDA recipients)
    await createAtaInterface(rpc, payer, mint, vaultPda, true);
    const vaultLightAta = getAssociatedTokenAddressInterface(mint, vaultPda, true);

    const fundIx = createLightTokenTransferInstruction(
        lightTokenAta,
        vaultLightAta,
        payer.publicKey,
        500_000
    );
    const { blockhash: bh1 } = await rpc.getLatestBlockhash();
    const fundTx = buildAndSignTx([fundIx], payer, bh1, []);
    await sendAndConfirmTx(rpc, fundTx);

    // 5. Fund vault with SOL for inner transaction fees
    const solTx = new Transaction().add(
        SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: vaultPda,
            lamports: 10_000_000, // 0.01 SOL
        })
    );
    solTx.recentBlockhash = (await rpc.getLatestBlockhash()).blockhash;
    solTx.feePayer = payer.publicKey;
    solTx.sign(payer);
    const fundSolSig = await rpc.sendRawTransaction(solTx.serialize());
    await rpc.confirmTransaction(fundSolSig, "confirmed");

    // 6. Unwrap Light Tokens to SPL inside the vault via vault transaction
    const vaultSplAta = await getAssociatedTokenAddress(
        mint,
        vaultPda,
        true,
        TOKEN_PROGRAM_ID
    );
    const unwrapIxBatches = await createUnwrapInstructions(
        rpc,
        vaultSplAta,
        vaultPda,
        mint,
        200_000
    );

    const multisigAccount = await multisig.accounts.Multisig.fromAccountAddress(
        rpc,
        multisigPda
    );
    let txIndex =
        BigInt(multisigAccount.transactionIndex.toString()) + 1n;

    for (const ixs of unwrapIxBatches) {
        const unwrapMessage = new TransactionMessage({
            payerKey: vaultPda,
            recentBlockhash: (await rpc.getLatestBlockhash()).blockhash,
            instructions: ixs,
        });

        const vtSig = await multisig.rpc.vaultTransactionCreate({
            connection: rpc,
            feePayer: payer,
            multisigPda,
            transactionIndex: txIndex,
            creator: payer.publicKey,
            vaultIndex: 0,
            ephemeralSigners: 0,
            transactionMessage: unwrapMessage,
        });
        await rpc.confirmTransaction(vtSig, "confirmed");

        const proposalSig = await multisig.rpc.proposalCreate({
            connection: rpc,
            feePayer: payer,
            multisigPda,
            transactionIndex: txIndex,
            creator: payer,
        });
        await rpc.confirmTransaction(proposalSig, "confirmed");

        const approveSig = await multisig.rpc.proposalApprove({
            connection: rpc,
            feePayer: payer,
            multisigPda,
            transactionIndex: txIndex,
            member: payer,
        });
        await rpc.confirmTransaction(approveSig, "confirmed");

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

    // 7. Transfer SPL tokens from vault's SPL ATA to a recipient
    const recipient = Keypair.generate();
    const recipientSplAta = await createAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        recipient.publicKey,
        undefined,
        TOKEN_PROGRAM_ID
    );

    const transferIx = createTransferInstruction(
        vaultSplAta,
        recipientSplAta,
        vaultPda,
        100_000,
        [],
        TOKEN_PROGRAM_ID
    );

    const transferMessage = new TransactionMessage({
        payerKey: vaultPda,
        recentBlockhash: (await rpc.getLatestBlockhash()).blockhash,
        instructions: [transferIx],
    });

    const vtSig2 = await multisig.rpc.vaultTransactionCreate({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        creator: payer.publicKey,
        vaultIndex: 0,
        ephemeralSigners: 0,
        transactionMessage: transferMessage,
    });
    await rpc.confirmTransaction(vtSig2, "confirmed");

    const proposalSig2 = await multisig.rpc.proposalCreate({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        creator: payer,
    });
    await rpc.confirmTransaction(proposalSig2, "confirmed");

    const approveSig2 = await multisig.rpc.proposalApprove({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        member: payer,
    });
    await rpc.confirmTransaction(approveSig2, "confirmed");

    const sig = await multisig.rpc.vaultTransactionExecute({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        member: payer.publicKey,
        signers: [payer],
    });

    console.log("Tx:", sig);
})();
