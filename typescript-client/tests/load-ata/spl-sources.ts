import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { loadAta } from "@lightprotocol/compressed-token/unified";
import {
    rpc,
    payer,
    createMultipleCompressed,
    createSplMint,
    createLightAtaWithBalance,
    getSplAtaWithBalance,
    getSplAtaBalance,
    waitForCompressedCount,
    getLightAtaBalance,
    getAssociatedTokenAddressInterface,
    assert,
    logScenario,
    TOKEN_PROGRAM_ID,
} from "./setup.js";

const AMOUNT_EACH = 100n;
const SPL_AMOUNT = 1000n;

(async function () {
    console.log("\n=== loadAta: SPL mint sources ===\n");

    // ── Setup ──────────────────────────────────────────────

    console.log("  Setting up scenarios...\n");

    // B1: 0 cold, 1000 SPL, no Light ATA
    const b1Mint = await createSplMint();
    const b1Owner = Keypair.generate();
    const b1SplAta = await getSplAtaWithBalance(b1Mint, b1Owner.publicKey, SPL_AMOUNT);

    // B2: 1 cold, 1000 SPL, no Light ATA
    const b2Mint = await createSplMint();
    const b2Owner = Keypair.generate();
    await createMultipleCompressed(b2Mint, b2Owner.publicKey, 1, AMOUNT_EACH);
    const b2SplAta = await getSplAtaWithBalance(b2Mint, b2Owner.publicKey, SPL_AMOUNT);

    // B3: 4 cold, 1000 SPL, no Light ATA
    const b3Mint = await createSplMint();
    const b3Owner = Keypair.generate();
    await createMultipleCompressed(b3Mint, b3Owner.publicKey, 4, AMOUNT_EACH);
    const b3SplAta = await getSplAtaWithBalance(b3Mint, b3Owner.publicKey, SPL_AMOUNT);

    // B4: 8 cold, 1000 SPL, no Light ATA
    const b4Mint = await createSplMint();
    const b4Owner = Keypair.generate();
    await createMultipleCompressed(b4Mint, b4Owner.publicKey, 8, AMOUNT_EACH);
    const b4SplAta = await getSplAtaWithBalance(b4Mint, b4Owner.publicKey, SPL_AMOUNT);

    // B5: 4 cold, 1000 SPL, Light ATA has 500
    const b5Mint = await createSplMint();
    const b5Owner = Keypair.generate();
    await createLightAtaWithBalance(b5Mint, b5Owner, 500n);
    await createMultipleCompressed(b5Mint, b5Owner.publicKey, 4, AMOUNT_EACH);
    const b5SplAta = await getSplAtaWithBalance(b5Mint, b5Owner.publicKey, SPL_AMOUNT);

    // B6: 4 cold, 0 SPL, no Light ATA
    const b6Mint = await createSplMint();
    const b6Owner = Keypair.generate();
    await createMultipleCompressed(b6Mint, b6Owner.publicKey, 4, AMOUNT_EACH);
    await getSplAtaWithBalance(b6Mint, b6Owner.publicKey, 0n);

    // B7: 0 cold, 0 SPL, no Light ATA
    const b7Mint = await createSplMint();
    const b7Owner = Keypair.generate();
    await getSplAtaWithBalance(b7Mint, b7Owner.publicKey, 0n);

    // ── Tests ──────────────────────────────────────────────

    console.log("  Running tests...\n");

    // B1: 0 cold, 1000 SPL, no Light ATA -> creates ATA, wraps SPL
    {
        const ata = getAssociatedTokenAddressInterface(b1Mint, b1Owner.publicKey);
        const tx = await loadAta(rpc, ata, b1Owner, b1Mint, payer);
        assert(tx !== null, "B1: expected transaction signature");

        await waitForCompressedCount(b1Owner.publicKey, b1Mint, 0);

        const splBalance = await getSplAtaBalance(b1SplAta, TOKEN_PROGRAM_ID);
        assert(splBalance === 0n, `B1: expected SPL balance 0, got ${splBalance}`);

        const lightBalance = await getLightAtaBalance(ata, b1Owner.publicKey, b1Mint);
        logScenario("B1: 0 cold + 1000 SPL, no ATA", SPL_AMOUNT, lightBalance);
    }

    // B2: 1 cold, 1000 SPL, no Light ATA -> creates ATA, loads 1 + wraps
    {
        const ata = getAssociatedTokenAddressInterface(b2Mint, b2Owner.publicKey);
        const tx = await loadAta(rpc, ata, b2Owner, b2Mint, payer);
        assert(tx !== null, "B2: expected transaction signature");

        await waitForCompressedCount(b2Owner.publicKey, b2Mint, 0);

        const splBalance = await getSplAtaBalance(b2SplAta, TOKEN_PROGRAM_ID);
        assert(splBalance === 0n, `B2: expected SPL balance 0, got ${splBalance}`);

        const lightBalance = await getLightAtaBalance(ata, b2Owner.publicKey, b2Mint);
        logScenario("B2: 1 cold + 1000 SPL, no ATA", 1n * AMOUNT_EACH + SPL_AMOUNT, lightBalance);
    }

    // B3: 4 cold, 1000 SPL, no Light ATA -> creates ATA, loads 4 + wraps
    {
        const ata = getAssociatedTokenAddressInterface(b3Mint, b3Owner.publicKey);
        const tx = await loadAta(rpc, ata, b3Owner, b3Mint, payer);
        assert(tx !== null, "B3: expected transaction signature");

        await waitForCompressedCount(b3Owner.publicKey, b3Mint, 0);

        const splBalance = await getSplAtaBalance(b3SplAta, TOKEN_PROGRAM_ID);
        assert(splBalance === 0n, `B3: expected SPL balance 0, got ${splBalance}`);

        const lightBalance = await getLightAtaBalance(ata, b3Owner.publicKey, b3Mint);
        logScenario("B3: 4 cold + 1000 SPL, no ATA", 4n * AMOUNT_EACH + SPL_AMOUNT, lightBalance);
    }

    // B4: 8 cold, 1000 SPL, no Light ATA -> creates ATA, loads 8 + wraps
    {
        const ata = getAssociatedTokenAddressInterface(b4Mint, b4Owner.publicKey);
        const tx = await loadAta(rpc, ata, b4Owner, b4Mint, payer);
        assert(tx !== null, "B4: expected transaction signature");

        await waitForCompressedCount(b4Owner.publicKey, b4Mint, 0);

        const splBalance = await getSplAtaBalance(b4SplAta, TOKEN_PROGRAM_ID);
        assert(splBalance === 0n, `B4: expected SPL balance 0, got ${splBalance}`);

        const lightBalance = await getLightAtaBalance(ata, b4Owner.publicKey, b4Mint);
        logScenario("B4: 8 cold + 1000 SPL, no ATA", 8n * AMOUNT_EACH + SPL_AMOUNT, lightBalance);
    }

    // B5: 4 cold, 1000 SPL, Light ATA has 500 -> loads 4 + wraps, total = 500 + 400 + 1000
    {
        const ata = getAssociatedTokenAddressInterface(b5Mint, b5Owner.publicKey);
        const tx = await loadAta(rpc, ata, b5Owner, b5Mint, payer);
        assert(tx !== null, "B5: expected transaction signature");

        await waitForCompressedCount(b5Owner.publicKey, b5Mint, 0);

        const splBalance = await getSplAtaBalance(b5SplAta, TOKEN_PROGRAM_ID);
        assert(splBalance === 0n, `B5: expected SPL balance 0, got ${splBalance}`);

        const lightBalance = await getLightAtaBalance(ata, b5Owner.publicKey, b5Mint);
        logScenario("B5: 4 cold + 1000 SPL, ATA has 500", 500n + 4n * AMOUNT_EACH + SPL_AMOUNT, lightBalance);
    }

    // B6: 4 cold, 0 SPL, no Light ATA -> creates ATA, loads 4 cold only
    {
        const ata = getAssociatedTokenAddressInterface(b6Mint, b6Owner.publicKey);
        const tx = await loadAta(rpc, ata, b6Owner, b6Mint, payer);
        assert(tx !== null, "B6: expected transaction signature");

        await waitForCompressedCount(b6Owner.publicKey, b6Mint, 0);

        const lightBalance = await getLightAtaBalance(ata, b6Owner.publicKey, b6Mint);
        logScenario("B6: 4 cold + 0 SPL, no ATA", 4n * AMOUNT_EACH, lightBalance);
    }

    // B7: 0 cold, 0 SPL, no Light ATA -> returns null
    {
        const ata = getAssociatedTokenAddressInterface(b7Mint, b7Owner.publicKey);
        const tx = await loadAta(rpc, ata, b7Owner, b7Mint, payer);
        assert(tx === null, "B7: expected null (nothing to load)");

        const lightBalance = await getLightAtaBalance(ata, b7Owner.publicKey, b7Mint);
        logScenario("B7: 0 cold + 0 SPL, no ATA", 0n, lightBalance);
    }

    console.log("\n=== All SPL source scenarios passed ===\n");
})();
