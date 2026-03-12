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

    // 3. Fund vault with Light Tokens
    await createAtaInterface(rpc, payer, mint, vaultPda, true);
    const vaultAta = getAssociatedTokenAddressInterface(mint, vaultPda, true);

    const fundIx = createLightTokenTransferInstruction(
        payerAta,
        vaultAta,
        payer.publicKey,
        500_000
    );
    const { blockhash: bh1 } = await rpc.getLatestBlockhash();
    const fundTx = buildAndSignTx([fundIx], payer, bh1, []);
    await sendAndConfirmTx(rpc, fundTx);

    // 4. Fund vault with SOL for inner transaction fees
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

    // 5. Build transfer instruction FROM vault to a recipient
    const recipient = Keypair.generate();
    await createAtaInterface(rpc, payer, mint, recipient.publicKey);
    const recipientAta = getAssociatedTokenAddressInterface(
        mint,
        recipient.publicKey
    );

    // createLightTokenTransferInstruction works with off-curve PDA owners
    const transferIx = createLightTokenTransferInstruction(
        vaultAta,
        recipientAta,
        vaultPda,
        100_000,
        vaultPda
    );

    // 6. Wrap in a Squads vault transaction
    const multisigAccount = await multisig.accounts.Multisig.fromAccountAddress(
        rpc,
        multisigPda
    );
    const txIndex = BigInt(multisigAccount.transactionIndex.toString()) + 1n;

    const transferMessage = new TransactionMessage({
        payerKey: vaultPda,
        recentBlockhash: (await rpc.getLatestBlockhash()).blockhash,
        instructions: [transferIx],
    });

    const vtSig = await multisig.rpc.vaultTransactionCreate({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        creator: payer.publicKey,
        vaultIndex: 0,
        ephemeralSigners: 0,
        transactionMessage: transferMessage,
    });
    await rpc.confirmTransaction(vtSig, "confirmed");

    // 7. Propose, approve, execute
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

    const execSig = await multisig.rpc.vaultTransactionExecute({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex: txIndex,
        member: payer.publicKey,
        signers: [payer],
    });

    console.log("Vault:", vaultPda.toBase58());
    console.log("Recipient:", recipient.publicKey.toBase58());
    console.log("Tx:", execSig);
})();
