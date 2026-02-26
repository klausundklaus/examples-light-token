import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
  createMockRpc,
  createMockSignTransaction,
  TEST_KEYPAIR,
} from './mock-rpc';

const mockRpc = createMockRpc();

vi.mock('@lightprotocol/stateless.js', () => ({
  createRpc: () => mockRpc,
}));

const dummyIx = new TransactionInstruction({
  keys: [],
  programId: new PublicKey('11111111111111111111111111111111'),
  data: Buffer.alloc(0),
});

const mockCreateUnwrapInstructions = vi.fn();

vi.mock('@lightprotocol/compressed-token/unified', () => ({
  createUnwrapInstructions: (...args: unknown[]) =>
    mockCreateUnwrapInstructions(...args),
}));

// Keep real getAssociatedTokenAddressSync
vi.mock('@solana/spl-token', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@solana/spl-token')>();
  return { ...actual };
});

import { useUnwrap } from '../useUnwrap';

const signTransaction = createMockSignTransaction();

const TOKEN_2022_PROGRAM_ID = new PublicKey(
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb',
);

beforeEach(() => {
  vi.clearAllMocks();
  mockRpc.getLatestBlockhash.mockResolvedValue({
    blockhash: '11111111111111111111111111111111',
    lastValidBlockHeight: 100,
  });
  mockRpc.sendRawTransaction.mockResolvedValue('tx-sig-unwrap');
  signTransaction.mockImplementation(async (tx: any) => {
    tx.partialSign(TEST_KEYPAIR);
    return tx;
  });
  // Default: mint account exists with T22 as owner
  mockRpc.getAccountInfo.mockResolvedValue({ owner: TOKEN_2022_PROGRAM_ID });
  // Default: single batch with one instruction
  mockCreateUnwrapInstructions.mockResolvedValue([[dummyIx]]);
});

describe('useUnwrap', () => {
  it('builds, signs, and sends an unwrap transaction', async () => {
    const { result } = renderHook(() => useUnwrap());

    let sig: string | undefined;
    await act(async () => {
      sig = await result.current.unwrap({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          amount: 2,
          decimals: 9,
        },
        signTransaction,
      });
    });

    expect(sig).toBe('tx-sig-unwrap');
    expect(mockCreateUnwrapInstructions).toHaveBeenCalledOnce();
    expect(signTransaction).toHaveBeenCalledOnce();
    expect(mockRpc.sendRawTransaction).toHaveBeenCalledOnce();
  });

  it('handles multiple instruction batches', async () => {
    mockCreateUnwrapInstructions.mockResolvedValue([
      [dummyIx],
      [dummyIx],
    ]);

    const { result } = renderHook(() => useUnwrap());

    await act(async () => {
      await result.current.unwrap({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          amount: 1,
        },
        signTransaction,
      });
    });

    expect(signTransaction).toHaveBeenCalledTimes(2);
    expect(mockRpc.sendRawTransaction).toHaveBeenCalledTimes(2);
  });

  it('propagates SDK errors', async () => {
    mockCreateUnwrapInstructions.mockRejectedValue(
      new Error('Unwrap failed'),
    );

    const { result } = renderHook(() => useUnwrap());

    await expect(
      act(async () => {
        await result.current.unwrap({
          params: {
            ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
            mint: PublicKey.unique().toBase58(),
            amount: 1,
          },
          signTransaction,
        });
      }),
    ).rejects.toThrow('Unwrap failed');

    expect(result.current.isLoading).toBe(false);
  });

  it('clears isLoading after completion', async () => {
    const { result } = renderHook(() => useUnwrap());

    await act(async () => {
      await result.current.unwrap({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          amount: 1,
        },
        signTransaction,
      });
    });

    expect(result.current.isLoading).toBe(false);
  });
});
