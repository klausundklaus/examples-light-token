import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import {
    createLoadAtaInstructions,
    createTransferInterfaceInstruction,
} from "@lightprotocol/compressed-token/unified";
import {
    buildAndSignTx,
    sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import {
    rpc,
    payer,
    createMultipleCompressed,
    createSplMint,
    createT22Mint,
    TOKEN_2022_PROGRAM_ID,
    getSplAtaWithBalance,
    getT22AtaWithBalance,
    getLightAtaBalance,
    getAssociatedTokenAddressInterface,
    createAtaInterface,
    assert,
    logScenario,
} from "./setup.js";

const AMOUNT_EACH = 100n;
const TRANSFER_AMOUNT = 50;

(async function () {
    console.log("\n=== Load + Transfer in Single Tx ===\n");

    // ── Setup ──────────────────────────────────────────────

    console.log("  Setting up scenarios...\n");

    // D1: 1 cold (SPL mint)
    const d1Mint = await createSplMint();
    const d1Sender = Keypair.generate();
    const d1Recipient = Keypair.generate();
    await createMultipleCompressed(d1Mint, d1Sender.publicKey, 1, AMOUNT_EACH);
    await createAtaInterface(rpc, payer, d1Mint, d1Recipient.publicKey);

    // D2: 4 cold (SPL mint)
    const d2Mint = await createSplMint();
    const d2Sender = Keypair.generate();
    const d2Recipient = Keypair.generate();
    await createMultipleCompressed(d2Mint, d2Sender.publicKey, 4, AMOUNT_EACH);
    await createAtaInterface(rpc, payer, d2Mint, d2Recipient.publicKey);

    // D3: 2 cold + 1000 SPL
    const d3Mint = await createSplMint();
    const d3Sender = Keypair.generate();
    const d3Recipient = Keypair.generate();
    await createMultipleCompressed(d3Mint, d3Sender.publicKey, 2, AMOUNT_EACH);
    await getSplAtaWithBalance(d3Mint, d3Sender.publicKey, 1000n);
    await createAtaInterface(rpc, payer, d3Mint, d3Recipient.publicKey);

    // D4: 2 cold + 1000 T22
    const d4Mint = await createT22Mint();
    const d4Sender = Keypair.generate();
    const d4Recipient = Keypair.generate();
    await createMultipleCompressed(d4Mint, d4Sender.publicKey, 2, AMOUNT_EACH, TOKEN_2022_PROGRAM_ID);
    await getT22AtaWithBalance(d4Mint, d4Sender.publicKey, 1000n);
    await createAtaInterface(rpc, payer, d4Mint, d4Recipient.publicKey);

    // ── Tests ──────────────────────────────────────────────

    console.log("  Running tests...\n");

    // D1: 1 cold (SPL mint) -> load + transfer in 1 tx
    {
        const senderAta = getAssociatedTokenAddressInterface(d1Mint, d1Sender.publicKey);
        const recipientAta = getAssociatedTokenAddressInterface(d1Mint, d1Recipient.publicKey);

        const loadIxs = await createLoadAtaInstructions(
            rpc, senderAta, d1Sender.publicKey, d1Mint, payer.publicKey,
        );
        assert(loadIxs.length > 0, "D1: should have load instructions");

        const transferIx = createTransferInterfaceInstruction(
            senderAta, recipientAta, d1Sender.publicKey, TRANSFER_AMOUNT,
        );

        const blockhash = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [...loadIxs, transferIx], payer, blockhash.blockhash, [d1Sender],
        );
        await sendAndConfirmTx(rpc, tx);

        const senderBalance = await getLightAtaBalance(senderAta, d1Sender.publicKey, d1Mint);
        const recipientBalance = await getLightAtaBalance(recipientAta, d1Recipient.publicKey, d1Mint);

        logScenario("D1: sender balance after load+transfer", 1n * AMOUNT_EACH - BigInt(TRANSFER_AMOUNT), senderBalance);
        logScenario("D1: recipient balance after load+transfer", BigInt(TRANSFER_AMOUNT), recipientBalance);
    }

    // D2: 4 cold (SPL mint) -> load + transfer in 1 tx
    {
        const senderAta = getAssociatedTokenAddressInterface(d2Mint, d2Sender.publicKey);
        const recipientAta = getAssociatedTokenAddressInterface(d2Mint, d2Recipient.publicKey);

        const loadIxs = await createLoadAtaInstructions(
            rpc, senderAta, d2Sender.publicKey, d2Mint, payer.publicKey,
        );
        assert(loadIxs.length > 0, "D2: should have load instructions");

        const transferIx = createTransferInterfaceInstruction(
            senderAta, recipientAta, d2Sender.publicKey, TRANSFER_AMOUNT,
        );

        const blockhash = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [...loadIxs, transferIx], payer, blockhash.blockhash, [d2Sender],
        );
        await sendAndConfirmTx(rpc, tx);

        const senderBalance = await getLightAtaBalance(senderAta, d2Sender.publicKey, d2Mint);
        const recipientBalance = await getLightAtaBalance(recipientAta, d2Recipient.publicKey, d2Mint);

        logScenario("D2: sender (4 cold load+transfer)", 4n * AMOUNT_EACH - BigInt(TRANSFER_AMOUNT), senderBalance);
        logScenario("D2: recipient (4 cold load+transfer)", BigInt(TRANSFER_AMOUNT), recipientBalance);
    }

    // D3: SPL ATA + 2 cold -> load + wrap + transfer in 1 tx
    {
        const senderAta = getAssociatedTokenAddressInterface(d3Mint, d3Sender.publicKey);
        const recipientAta = getAssociatedTokenAddressInterface(d3Mint, d3Recipient.publicKey);

        const loadIxs = await createLoadAtaInstructions(
            rpc, senderAta, d3Sender.publicKey, d3Mint, payer.publicKey,
        );
        assert(loadIxs.length > 0, "D3: should have load instructions");

        const transferIx = createTransferInterfaceInstruction(
            senderAta, recipientAta, d3Sender.publicKey, TRANSFER_AMOUNT,
        );

        const blockhash = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [...loadIxs, transferIx], payer, blockhash.blockhash, [d3Sender],
        );
        await sendAndConfirmTx(rpc, tx);

        const senderBalance = await getLightAtaBalance(senderAta, d3Sender.publicKey, d3Mint);
        const recipientBalance = await getLightAtaBalance(recipientAta, d3Recipient.publicKey, d3Mint);
        const totalLoaded = 2n * AMOUNT_EACH + 1000n;

        logScenario("D3: sender (SPL+2 cold load+transfer)", totalLoaded - BigInt(TRANSFER_AMOUNT), senderBalance);
        logScenario("D3: recipient (SPL+2 cold load+transfer)", BigInt(TRANSFER_AMOUNT), recipientBalance);
    }

    // D4: T22 ATA + 2 cold -> load + wrap + transfer in 1 tx
    {
        const senderAta = getAssociatedTokenAddressInterface(d4Mint, d4Sender.publicKey);
        const recipientAta = getAssociatedTokenAddressInterface(d4Mint, d4Recipient.publicKey);

        const loadIxs = await createLoadAtaInstructions(
            rpc, senderAta, d4Sender.publicKey, d4Mint, payer.publicKey,
        );
        assert(loadIxs.length > 0, "D4: should have load instructions");

        const transferIx = createTransferInterfaceInstruction(
            senderAta, recipientAta, d4Sender.publicKey, TRANSFER_AMOUNT,
        );

        const blockhash = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [...loadIxs, transferIx], payer, blockhash.blockhash, [d4Sender],
        );
        await sendAndConfirmTx(rpc, tx);

        const senderBalance = await getLightAtaBalance(senderAta, d4Sender.publicKey, d4Mint);
        const recipientBalance = await getLightAtaBalance(recipientAta, d4Recipient.publicKey, d4Mint);
        const totalLoaded = 2n * AMOUNT_EACH + 1000n;

        logScenario("D4: sender (T22+2 cold load+transfer)", totalLoaded - BigInt(TRANSFER_AMOUNT), senderBalance);
        logScenario("D4: recipient (T22+2 cold load+transfer)", BigInt(TRANSFER_AMOUNT), recipientBalance);
    }

    console.log("\n=== All Load+Transfer Scenarios Passed ===\n");
})();
