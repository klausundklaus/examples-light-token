import { Keypair, PublicKey } from "@solana/web3.js";
import { createRpc, Rpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    compress,
    createAtaInterface,
    loadAta,
    getAssociatedTokenAddressInterface,
    getAtaInterface,
} from "@lightprotocol/compressed-token";
import {
    createAssociatedTokenAccount,
    getOrCreateAssociatedTokenAccount,
    mintTo,
    getAccount,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { homedir } from "os";
import { readFileSync } from "fs";

// localnet RPC
export const rpc: Rpc = createRpc();

export const payer = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, "utf8")),
    ),
);

// Create N compressed token accounts for owner, each with `amountEach`.
// Mints tokens to payer's ATA, then compresses into cold accounts owned by `owner`.
// Waits for the indexer to reflect all new accounts before returning.
export async function createMultipleCompressed(
    mint: PublicKey,
    owner: PublicKey,
    count: number,
    amountEach: bigint,
    tokenProgramId: PublicKey = TOKEN_PROGRAM_ID,
): Promise<void> {
    const before = await getCompressedCount(owner, mint);

    // Get or create payer's ATA for this mint
    const payerAta = await getOrCreateAssociatedTokenAccount(
        rpc, payer, mint, payer.publicKey, undefined, undefined, undefined, tokenProgramId,
    );

    // Mint total amount to payer's ATA
    const totalAmount = amountEach * BigInt(count);
    await mintTo(rpc, payer, mint, payerAta.address, payer, totalAmount, [], undefined, tokenProgramId);

    // Compress into N cold accounts owned by `owner`
    const amounts = Array(count).fill(Number(amountEach));
    const recipients = Array(count).fill(owner);
    await compress(rpc, payer, mint, amounts, payer, payerAta.address, recipients);

    const expected = before + count;
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
        const indexed = await getCompressedCount(owner, mint);
        if (indexed >= expected) return;
        await new Promise((r) => setTimeout(r, 200));
    }
    throw new Error(
        `Indexer timeout: expected ${expected} compressed accounts, got ${await getCompressedCount(owner, mint)}`,
    );
}

// Create SPL ATA and mint tokens into it
export async function getSplAtaWithBalance(
    mint: PublicKey,
    owner: PublicKey,
    amount: bigint,
): Promise<PublicKey> {
    const ata = await createAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        owner,
        undefined,
        TOKEN_PROGRAM_ID,
    );
    if (amount > 0n) {
        await mintTo(rpc, payer, mint, ata, payer, amount, [], undefined, TOKEN_PROGRAM_ID);
    }
    return ata;
}

// Create T22 ATA and mint tokens into it
export async function getT22AtaWithBalance(
    mint: PublicKey,
    owner: PublicKey,
    amount: bigint,
): Promise<PublicKey> {
    const ata = await createAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        owner,
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );
    if (amount > 0n) {
        await mintTo(rpc, payer, mint, ata, payer, amount, [], undefined, TOKEN_2022_PROGRAM_ID);
    }
    return ata;
}

// Count compressed token accounts for owner + mint
export async function getCompressedCount(
    owner: PublicKey,
    mint: PublicKey,
): Promise<number> {
    const result = await rpc.getCompressedTokenAccountsByOwner(owner, {
        mint,
    });
    return result.items.length;
}

// Poll indexer until compressed count reaches expected value.
// Use after loadAta to wait for consumed accounts to disappear from the index.
export async function waitForCompressedCount(
    owner: PublicKey,
    mint: PublicKey,
    expected: number,
    timeoutMs = 30_000,
): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
        const count = await getCompressedCount(owner, mint);
        if (count === expected) return;
        await new Promise((r) => setTimeout(r, 200));
    }
    const actual = await getCompressedCount(owner, mint);
    throw new Error(
        `Indexer timeout: expected ${expected} compressed accounts, got ${actual}`,
    );
}

// Get Light ATA balance (returns 0n if account doesn't exist)
export async function getLightAtaBalance(
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
): Promise<bigint> {
    try {
        const account = await getAtaInterface(rpc, ata, owner, mint);
        return account.parsed.amount;
    } catch {
        return 0n;
    }
}

// Get SPL/T22 ATA balance
export async function getSplAtaBalance(
    ata: PublicKey,
    programId: PublicKey = TOKEN_PROGRAM_ID,
): Promise<bigint> {
    try {
        const account = await getAccount(rpc, ata, undefined, programId);
        return account.amount;
    } catch {
        return 0n;
    }
}

// Simple assert
export function assert(condition: boolean, msg: string): void {
    if (!condition) {
        throw new Error(`FAIL: ${msg}`);
    }
}

// Log scenario result
export function logScenario(
    name: string,
    expected: bigint,
    actual: bigint,
): void {
    const pass = expected === actual;
    const status = pass ? "PASS" : "FAIL";
    console.log(
        `  [${status}] ${name}: expected=${expected}, actual=${actual}`,
    );
    if (!pass) {
        throw new Error(
            `${name}: expected=${expected}, actual=${actual}`,
        );
    }
}

// Create a fresh SPL mint with Light interface PDA
export async function createSplMint(): Promise<PublicKey> {
    const mintKeypair = Keypair.generate();
    const { mint } = await createMintInterface(
        rpc,
        payer,
        payer,
        null,
        9,
        mintKeypair,
        undefined,
        TOKEN_PROGRAM_ID,
    );
    return mint;
}

// Create a fresh T22 mint with Light interface PDA
export async function createT22Mint(): Promise<PublicKey> {
    const mintKeypair = Keypair.generate();
    const { mint } = await createMintInterface(
        rpc,
        payer,
        payer,
        null,
        9,
        mintKeypair,
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );
    return mint;
}

// Create Light ATA and optionally pre-fill it with balance.
// When amount is provided: creates a temporary cold account, then loads it into the ATA.
// IMPORTANT: Call this BEFORE creating test cold accounts and SPL/T22 ATAs,
// otherwise loadAta in setup would consume them.
export async function createLightAtaWithBalance(
    mint: PublicKey,
    owner: Keypair,
    amount?: bigint,
    tokenProgramId: PublicKey = TOKEN_PROGRAM_ID,
): Promise<PublicKey> {
    await createAtaInterface(rpc, payer, mint, owner.publicKey);
    const ata = getAssociatedTokenAddressInterface(mint, owner.publicKey);
    if (amount && amount > 0n) {
        await createMultipleCompressed(mint, owner.publicKey, 1, amount, tokenProgramId);
        await loadAta(rpc, ata, owner, mint, payer);
        await waitForCompressedCount(owner.publicKey, mint, 0);
    }
    return ata;
}

// Re-export commonly used functions
export {
    getAssociatedTokenAddressInterface,
    createAtaInterface,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
};
