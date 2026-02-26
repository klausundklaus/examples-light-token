import { vi } from 'vitest';
import { Keypair, PublicKey, Transaction } from '@solana/web3.js';

/** Shared test keypair — use TEST_KEYPAIR.publicKey as ownerPublicKey in tests. */
export const TEST_KEYPAIR = Keypair.generate();

/** Creates a mock RPC object matching the shape returned by `createRpc`. */
export function createMockRpc() {
  return {
    getBalance: vi.fn().mockResolvedValue(1_000_000_000), // 1 SOL
    getTokenAccountsByOwner: vi.fn().mockResolvedValue({ value: [] }),
    getAccountInfo: vi.fn().mockResolvedValue(null),
    getCompressedTokenBalancesByOwnerV2: vi.fn().mockResolvedValue({
      value: { items: [] },
    }),
    getSignaturesForOwnerInterface: vi.fn().mockResolvedValue({
      signatures: [],
    }),
    getLatestBlockhash: vi.fn().mockResolvedValue({
      blockhash: '11111111111111111111111111111111',
      lastValidBlockHeight: 100,
    }),
    sendRawTransaction: vi.fn().mockResolvedValue('mock-signature-abc123'),
    confirmTransaction: vi.fn().mockResolvedValue({}),
  };
}

/** Builds a minimal SPL/T22 token account data buffer (72+ bytes). */
export function buildTokenAccountData(mint: PublicKey, amount: bigint): Buffer {
  const buf = Buffer.alloc(72);
  mint.toBuffer().copy(buf, 0); // bytes 0-31: mint
  // bytes 32-63: owner (unused in tests)
  buf.writeBigUInt64LE(amount, 64); // bytes 64-71: amount
  return buf;
}

/** Creates a mock signTransaction that signs with TEST_KEYPAIR. */
export function createMockSignTransaction() {
  return vi.fn().mockImplementation(async (tx: Transaction) => {
    tx.partialSign(TEST_KEYPAIR);
    return tx;
  });
}
