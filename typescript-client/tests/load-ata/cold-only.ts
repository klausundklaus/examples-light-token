import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { loadAta } from "@lightprotocol/compressed-token";
import {
    rpc,
    payer,
    createMultipleCompressed,
    createSplMint,
    createLightAtaWithBalance,
    waitForCompressedCount,
    getLightAtaBalance,
    getAssociatedTokenAddressInterface,
    assert,
    logScenario,
} from "./setup.js";

const AMOUNT_EACH = 100n;

(async function () {
    console.log("\n=== Cold-Only Scenarios (SPL Mint) ===\n");

    // ── Setup ──────────────────────────────────────────────
    // Create all mints, owners, and compressed accounts upfront.
    // By the time setup completes, the indexer has had time to
    // reflect all state — matching real-world usage where
    // accounts exist well before loadAta is called.

    console.log("  Setting up scenarios...\n");

    // A1: 1 cold, no ATA
    const a1Mint = await createSplMint();
    const a1Owner = Keypair.generate();
    await createMultipleCompressed(a1Mint, a1Owner.publicKey, 1, AMOUNT_EACH);

    // A2: 2 cold, no ATA
    const a2Mint = await createSplMint();
    const a2Owner = Keypair.generate();
    await createMultipleCompressed(a2Mint, a2Owner.publicKey, 2, AMOUNT_EACH);

    // A3: 4 cold, no ATA
    const a3Mint = await createSplMint();
    const a3Owner = Keypair.generate();
    await createMultipleCompressed(a3Mint, a3Owner.publicKey, 4, AMOUNT_EACH);

    // A4: 8 cold, no ATA
    const a4Mint = await createSplMint();
    const a4Owner = Keypair.generate();
    await createMultipleCompressed(a4Mint, a4Owner.publicKey, 8, AMOUNT_EACH);

    // A5: 1 cold, ATA exists (empty)
    const a5Mint = await createSplMint();
    const a5Owner = Keypair.generate();
    await createLightAtaWithBalance(a5Mint, a5Owner);
    await createMultipleCompressed(a5Mint, a5Owner.publicKey, 1, AMOUNT_EACH);

    // A6: 4 cold, ATA exists with 500
    const a6Mint = await createSplMint();
    const a6Owner = Keypair.generate();
    await createLightAtaWithBalance(a6Mint, a6Owner, 500n);
    await createMultipleCompressed(a6Mint, a6Owner.publicKey, 4, AMOUNT_EACH);

    // A7: 8 cold, ATA exists with 500
    const a7Mint = await createSplMint();
    const a7Owner = Keypair.generate();
    await createLightAtaWithBalance(a7Mint, a7Owner, 500n);
    await createMultipleCompressed(a7Mint, a7Owner.publicKey, 8, AMOUNT_EACH);

    // A8: 0 cold, no ATA
    const a8Mint = await createSplMint();
    const a8Owner = Keypair.generate();

    // A9: 0 cold, ATA exists with 500
    const a9Mint = await createSplMint();
    const a9Owner = Keypair.generate();
    await createLightAtaWithBalance(a9Mint, a9Owner, 500n);

    // ── Tests ──────────────────────────────────────────────

    console.log("  Running tests...\n");

    // A1: 1 cold, no ATA -> creates ATA, loads 1
    {
        const ata = getAssociatedTokenAddressInterface(a1Mint, a1Owner.publicKey);
        const tx = await loadAta(rpc, ata, a1Owner, a1Mint, payer);
        assert(tx !== null, "A1: should return tx signature");

        await waitForCompressedCount(a1Owner.publicKey, a1Mint, 0);
        const balance = await getLightAtaBalance(ata, a1Owner.publicKey, a1Mint);
        logScenario("A1: 1 cold, no ATA", 1n * AMOUNT_EACH, balance);
    }

    // A2: 2 cold, no ATA -> creates ATA, loads 2
    {
        const ata = getAssociatedTokenAddressInterface(a2Mint, a2Owner.publicKey);
        const tx = await loadAta(rpc, ata, a2Owner, a2Mint, payer);
        assert(tx !== null, "A2: should return tx signature");

        await waitForCompressedCount(a2Owner.publicKey, a2Mint, 0);
        const balance = await getLightAtaBalance(ata, a2Owner.publicKey, a2Mint);
        logScenario("A2: 2 cold, no ATA", 2n * AMOUNT_EACH, balance);
    }

    // A3: 4 cold, no ATA -> creates ATA, loads 4
    {
        const ata = getAssociatedTokenAddressInterface(a3Mint, a3Owner.publicKey);
        const tx = await loadAta(rpc, ata, a3Owner, a3Mint, payer);
        assert(tx !== null, "A3: should return tx signature");

        await waitForCompressedCount(a3Owner.publicKey, a3Mint, 0);
        const balance = await getLightAtaBalance(ata, a3Owner.publicKey, a3Mint);
        logScenario("A3: 4 cold, no ATA", 4n * AMOUNT_EACH, balance);
    }

    // A4: 8 cold, no ATA -> creates ATA, loads 8 (max)
    {
        const ata = getAssociatedTokenAddressInterface(a4Mint, a4Owner.publicKey);
        const tx = await loadAta(rpc, ata, a4Owner, a4Mint, payer);
        assert(tx !== null, "A4: should return tx signature");

        await waitForCompressedCount(a4Owner.publicKey, a4Mint, 0);
        const balance = await getLightAtaBalance(ata, a4Owner.publicKey, a4Mint);
        logScenario("A4: 8 cold, no ATA", 8n * AMOUNT_EACH, balance);
    }

    // A5: 1 cold, ATA exists (empty) -> loads 1
    {
        const ata = getAssociatedTokenAddressInterface(a5Mint, a5Owner.publicKey);
        const tx = await loadAta(rpc, ata, a5Owner, a5Mint, payer);
        assert(tx !== null, "A5: should return tx signature");

        await waitForCompressedCount(a5Owner.publicKey, a5Mint, 0);
        const balance = await getLightAtaBalance(ata, a5Owner.publicKey, a5Mint);
        logScenario("A5: 1 cold, ATA exists (empty)", 1n * AMOUNT_EACH, balance);
    }

    // A6: 4 cold, ATA exists with 500 -> loads 4, total = 500 + 4*100
    {
        const ata = getAssociatedTokenAddressInterface(a6Mint, a6Owner.publicKey);
        const tx = await loadAta(rpc, ata, a6Owner, a6Mint, payer);
        assert(tx !== null, "A6: should return tx signature");

        await waitForCompressedCount(a6Owner.publicKey, a6Mint, 0);
        const balance = await getLightAtaBalance(ata, a6Owner.publicKey, a6Mint);
        logScenario("A6: 4 cold, ATA has 500", 500n + 4n * AMOUNT_EACH, balance);
    }

    // A7: 8 cold, ATA exists with 500 -> loads 8, total = 500 + 8*100
    {
        const ata = getAssociatedTokenAddressInterface(a7Mint, a7Owner.publicKey);
        const tx = await loadAta(rpc, ata, a7Owner, a7Mint, payer);
        assert(tx !== null, "A7: should return tx signature");

        await waitForCompressedCount(a7Owner.publicKey, a7Mint, 0);
        const balance = await getLightAtaBalance(ata, a7Owner.publicKey, a7Mint);
        logScenario("A7: 8 cold, ATA has 500", 500n + 8n * AMOUNT_EACH, balance);
    }

    // A8: 0 cold, no ATA -> returns null, no ATA created
    {
        const ata = getAssociatedTokenAddressInterface(a8Mint, a8Owner.publicKey);
        const tx = await loadAta(rpc, ata, a8Owner, a8Mint, payer);
        assert(tx === null, "A8: should return null");

        const balance = await getLightAtaBalance(ata, a8Owner.publicKey, a8Mint);
        logScenario("A8: 0 cold, no ATA", 0n, balance);
    }

    // A9: 0 cold, ATA exists with 500 -> returns null, balance unchanged
    {
        const ata = getAssociatedTokenAddressInterface(a9Mint, a9Owner.publicKey);
        const tx = await loadAta(rpc, ata, a9Owner, a9Mint, payer);
        assert(tx === null, "A9: should return null");

        const balance = await getLightAtaBalance(ata, a9Owner.publicKey, a9Mint);
        logScenario("A9: 0 cold, ATA has 500", 500n, balance);
    }

    console.log("\n=== All Cold-Only Scenarios Passed ===\n");
})();
