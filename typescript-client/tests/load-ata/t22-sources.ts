import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { loadAta } from "@lightprotocol/compressed-token/unified";
import {
    rpc,
    payer,
    createMultipleCompressed,
    createT22Mint,
    createLightAtaWithBalance,
    getT22AtaWithBalance,
    getSplAtaBalance,
    waitForCompressedCount,
    getLightAtaBalance,
    getAssociatedTokenAddressInterface,
    assert,
    logScenario,
    TOKEN_2022_PROGRAM_ID,
} from "./setup.js";

const AMOUNT_EACH = 100n;
const T22_AMOUNT = 1000n;

(async function () {
    console.log("=== loadAta: Token-2022 mint sources ===\n");

    // ── Setup ──────────────────────────────────────────────

    console.log("  Setting up scenarios...\n");

    // C1: T22 ATA only, no cold, no existing Light ATA
    const c1Mint = await createT22Mint();
    const c1Owner = Keypair.generate();
    const c1T22Ata = await getT22AtaWithBalance(c1Mint, c1Owner.publicKey, T22_AMOUNT);

    // C2: 1 cold + T22 ATA, no existing Light ATA
    const c2Mint = await createT22Mint();
    const c2Owner = Keypair.generate();
    await createMultipleCompressed(c2Mint, c2Owner.publicKey, 1, AMOUNT_EACH, TOKEN_2022_PROGRAM_ID);
    const c2T22Ata = await getT22AtaWithBalance(c2Mint, c2Owner.publicKey, T22_AMOUNT);

    // C3: 4 cold + T22 ATA, no existing Light ATA
    const c3Mint = await createT22Mint();
    const c3Owner = Keypair.generate();
    await createMultipleCompressed(c3Mint, c3Owner.publicKey, 4, AMOUNT_EACH, TOKEN_2022_PROGRAM_ID);
    const c3T22Ata = await getT22AtaWithBalance(c3Mint, c3Owner.publicKey, T22_AMOUNT);

    // C4: 8 cold + T22 ATA + existing Light ATA with 500
    const c4Mint = await createT22Mint();
    const c4Owner = Keypair.generate();
    await createLightAtaWithBalance(c4Mint, c4Owner, 500n, TOKEN_2022_PROGRAM_ID);
    await createMultipleCompressed(c4Mint, c4Owner.publicKey, 8, AMOUNT_EACH, TOKEN_2022_PROGRAM_ID);
    const c4T22Ata = await getT22AtaWithBalance(c4Mint, c4Owner.publicKey, T22_AMOUNT);

    // C5: 4 cold + empty T22 ATA (0 balance), no existing Light ATA
    const c5Mint = await createT22Mint();
    const c5Owner = Keypair.generate();
    await createMultipleCompressed(c5Mint, c5Owner.publicKey, 4, AMOUNT_EACH, TOKEN_2022_PROGRAM_ID);
    const c5T22Ata = await getT22AtaWithBalance(c5Mint, c5Owner.publicKey, 0n);

    // ── Tests ──────────────────────────────────────────────

    console.log("  Running tests...\n");

    // C1: T22 ATA only -> creates ATA, wraps T22
    {
        const ata = getAssociatedTokenAddressInterface(c1Mint, c1Owner.publicKey);
        await loadAta(rpc, ata, c1Owner, c1Mint, payer);

        await waitForCompressedCount(c1Owner.publicKey, c1Mint, 0);

        const t22Balance = await getSplAtaBalance(c1T22Ata, TOKEN_2022_PROGRAM_ID);
        assert(t22Balance === 0n, "C1: T22 ATA should be drained");

        const lightBalance = await getLightAtaBalance(ata, c1Owner.publicKey, c1Mint);
        logScenario("C1: T22 ATA only (no cold)", T22_AMOUNT, lightBalance);
    }

    // C2: 1 cold + T22 ATA -> creates ATA, loads 1 + wraps
    {
        const ata = getAssociatedTokenAddressInterface(c2Mint, c2Owner.publicKey);
        await loadAta(rpc, ata, c2Owner, c2Mint, payer);

        await waitForCompressedCount(c2Owner.publicKey, c2Mint, 0);

        const t22Balance = await getSplAtaBalance(c2T22Ata, TOKEN_2022_PROGRAM_ID);
        assert(t22Balance === 0n, "C2: T22 ATA should be drained");

        const lightBalance = await getLightAtaBalance(ata, c2Owner.publicKey, c2Mint);
        logScenario("C2: 1 cold + T22 ATA", 1n * AMOUNT_EACH + T22_AMOUNT, lightBalance);
    }

    // C3: 4 cold + T22 ATA -> creates ATA, loads 4 + wraps
    {
        const ata = getAssociatedTokenAddressInterface(c3Mint, c3Owner.publicKey);
        await loadAta(rpc, ata, c3Owner, c3Mint, payer);

        await waitForCompressedCount(c3Owner.publicKey, c3Mint, 0);

        const t22Balance = await getSplAtaBalance(c3T22Ata, TOKEN_2022_PROGRAM_ID);
        assert(t22Balance === 0n, "C3: T22 ATA should be drained");

        const lightBalance = await getLightAtaBalance(ata, c3Owner.publicKey, c3Mint);
        logScenario("C3: 4 cold + T22 ATA", 4n * AMOUNT_EACH + T22_AMOUNT, lightBalance);
    }

    // C4: 8 cold + T22 ATA + existing 500 -> loads 8 + wraps
    {
        const ata = getAssociatedTokenAddressInterface(c4Mint, c4Owner.publicKey);
        await loadAta(rpc, ata, c4Owner, c4Mint, payer);

        await waitForCompressedCount(c4Owner.publicKey, c4Mint, 0);

        const t22Balance = await getSplAtaBalance(c4T22Ata, TOKEN_2022_PROGRAM_ID);
        assert(t22Balance === 0n, "C4: T22 ATA should be drained");

        const lightBalance = await getLightAtaBalance(ata, c4Owner.publicKey, c4Mint);
        logScenario("C4: 8 cold + T22 ATA + existing 500", 500n + 8n * AMOUNT_EACH + T22_AMOUNT, lightBalance);
    }

    // C5: 4 cold + empty T22 ATA -> creates ATA, loads 4 cold only
    {
        const ata = getAssociatedTokenAddressInterface(c5Mint, c5Owner.publicKey);
        await loadAta(rpc, ata, c5Owner, c5Mint, payer);

        await waitForCompressedCount(c5Owner.publicKey, c5Mint, 0);

        const t22Balance = await getSplAtaBalance(c5T22Ata, TOKEN_2022_PROGRAM_ID);
        assert(t22Balance === 0n, "C5: T22 ATA should remain 0");

        const lightBalance = await getLightAtaBalance(ata, c5Owner.publicKey, c5Mint);
        logScenario("C5: 4 cold + empty T22 ATA", 4n * AMOUNT_EACH, lightBalance);
    }

    console.log("\n=== All T22 source scenarios passed ===");
})();
