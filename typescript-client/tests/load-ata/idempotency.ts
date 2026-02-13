import "dotenv/config";
import { Keypair } from "@solana/web3.js";
import { loadAta } from "@lightprotocol/compressed-token/unified";
import {
    rpc,
    payer,
    createMultipleCompressed,
    createSplMint,
    getSplAtaWithBalance,
    getCompressedCount,
    waitForCompressedCount,
    getLightAtaBalance,
    getAssociatedTokenAddressInterface,
    assert,
    logScenario,
} from "./setup.js";

const AMOUNT_EACH = 100n;

(async function () {
    console.log("\n=== Idempotency Scenarios ===\n");

    // ── Setup ──────────────────────────────────────────────

    console.log("  Setting up scenarios...\n");

    // E1: 2 cold (SPL mint)
    const e1Mint = await createSplMint();
    const e1Owner = Keypair.generate();
    await createMultipleCompressed(e1Mint, e1Owner.publicKey, 2, AMOUNT_EACH);

    // E2: 2 cold (SPL mint) — second batch compressed mid-test
    const e2Mint = await createSplMint();
    const e2Owner = Keypair.generate();
    await createMultipleCompressed(e2Mint, e2Owner.publicKey, 2, AMOUNT_EACH);

    // E3: 1 cold + 1000 SPL
    const e3Mint = await createSplMint();
    const e3Owner = Keypair.generate();
    await createMultipleCompressed(e3Mint, e3Owner.publicKey, 1, AMOUNT_EACH);
    await getSplAtaWithBalance(e3Mint, e3Owner.publicKey, 1000n);

    // ── Tests ──────────────────────────────────────────────

    console.log("  Running tests...\n");

    // E1: Load, then load again immediately -> second returns null
    {
        const ata = getAssociatedTokenAddressInterface(e1Mint, e1Owner.publicKey);

        const tx1 = await loadAta(rpc, ata, e1Owner, e1Mint, payer);
        assert(tx1 !== null, "E1: first load should return tx");

        await waitForCompressedCount(e1Owner.publicKey, e1Mint, 0);

        const tx2 = await loadAta(rpc, ata, e1Owner, e1Mint, payer);
        assert(tx2 === null, "E1: second load should return null");

        const balance = await getLightAtaBalance(ata, e1Owner.publicKey, e1Mint);
        logScenario("E1: load then load again", 2n * AMOUNT_EACH, balance);
    }

    // E2: Load, create 2 more cold, load again -> second loads the 2 new ones
    {
        const ata = getAssociatedTokenAddressInterface(e2Mint, e2Owner.publicKey);

        const tx1 = await loadAta(rpc, ata, e2Owner, e2Mint, payer);
        assert(tx1 !== null, "E2: first load should return tx");

        const balanceAfterFirst = await getLightAtaBalance(ata, e2Owner.publicKey, e2Mint);
        assert(balanceAfterFirst === 2n * AMOUNT_EACH, "E2: first load balance check");

        // Mint 2 more compressed accounts mid-test (inherently sequential)
        await createMultipleCompressed(e2Mint, e2Owner.publicKey, 2, AMOUNT_EACH);

        const coldCount = await getCompressedCount(e2Owner.publicKey, e2Mint);
        assert(coldCount === 2, "E2: should have 2 new compressed accounts");

        const tx2 = await loadAta(rpc, ata, e2Owner, e2Mint, payer);
        assert(tx2 !== null, "E2: second load should return tx (new cold accounts)");

        await waitForCompressedCount(e2Owner.publicKey, e2Mint, 0);

        const balance = await getLightAtaBalance(ata, e2Owner.publicKey, e2Mint);
        logScenario("E2: load, add 2 more cold, load again", 4n * AMOUNT_EACH, balance);
    }

    // E3: Load SPL wrap, then load again -> second returns null
    {
        const ata = getAssociatedTokenAddressInterface(e3Mint, e3Owner.publicKey);

        const tx1 = await loadAta(rpc, ata, e3Owner, e3Mint, payer);
        assert(tx1 !== null, "E3: first load should return tx");

        await waitForCompressedCount(e3Owner.publicKey, e3Mint, 0);

        const tx2 = await loadAta(rpc, ata, e3Owner, e3Mint, payer);
        assert(tx2 === null, "E3: second load should return null");

        const balance = await getLightAtaBalance(ata, e3Owner.publicKey, e3Mint);
        logScenario("E3: load SPL+cold, then load again", 1n * AMOUNT_EACH + 1000n, balance);
    }

    console.log("\n=== All Idempotency Scenarios Passed ===\n");
})();
