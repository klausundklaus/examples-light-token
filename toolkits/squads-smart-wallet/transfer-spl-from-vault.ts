import "dotenv/config";
import { Keypair, TransactionMessage } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";
import {
    createUnwrapInstructions,
    transferInterface,
    wrap,
} from "@lightprotocol/compressed-token/unified";
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

    // 2. Mint SPL tokens, wrap into light-token ATA
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

    // 4. Fund vault with light tokens
    await createAtaInterface(rpc, payer, mint, vaultPda, true);
    await transferInterface(
        rpc,
        payer,
        lightTokenAta,
        mint,
        vaultPda,
        payer,
        500_000
    );

    // 5. Unwrap light tokens to SPL inside the vault via vault transaction.
    //    createUnwrapInstructions takes owner as PublicKey, so it works with PDAs.
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
        BigInt(multisigAccount.transactionIndex.toString()) + BigInt(1);

    for (const ixs of unwrapIxBatches) {
        const unwrapMessage = new TransactionMessage({
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
            transactionMessage: unwrapMessage,
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

    // 6. Transfer SPL tokens from vault's SPL ATA to a recipient
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
