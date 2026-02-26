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
  CTOKEN_PROGRAM_ID: new PublicKey('11111111111111111111111111111111'),
}));

const MOCK_LIGHT_ATA = PublicKey.unique();
const dummyIx = new TransactionInstruction({
  keys: [],
  programId: new PublicKey('11111111111111111111111111111111'),
  data: Buffer.alloc(0),
});

const mockGetSplInterfaceInfos = vi.fn();

vi.mock('@lightprotocol/compressed-token', () => ({
  getSplInterfaceInfos: (...args: unknown[]) =>
    mockGetSplInterfaceInfos(...args),
}));

vi.mock('@lightprotocol/compressed-token/unified', () => ({
  getAssociatedTokenAddressInterface: () => MOCK_LIGHT_ATA,
  createWrapInstruction: vi.fn(() => dummyIx),
  createAssociatedTokenAccountInterfaceIdempotentInstruction: vi.fn(() => dummyIx),
}));

// Partial mock: keep real getAssociatedTokenAddressSync, mock getAccount
const mockGetAccount = vi.fn();
vi.mock('@solana/spl-token', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@solana/spl-token')>();
  return { ...actual, getAccount: (...args: unknown[]) => mockGetAccount(...args) };
});

import { useWrap } from '../useWrap';

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
  mockRpc.sendRawTransaction.mockResolvedValue('tx-sig-wrap');
  signTransaction.mockImplementation(async (tx: any) => {
    tx.partialSign(TEST_KEYPAIR);
    return tx;
  });
  // Default: initialized SPL interface with T22
  mockGetSplInterfaceInfos.mockResolvedValue([
    { isInitialized: true, tokenProgram: TOKEN_2022_PROGRAM_ID },
  ]);
  // Default: ATA exists with sufficient balance
  mockGetAccount.mockResolvedValue({ amount: 10_000_000_000n });
});

describe('useWrap', () => {
  it('builds, signs, and sends a wrap transaction', async () => {
    const { result } = renderHook(() => useWrap());

    let sig: string | undefined;
    await act(async () => {
      sig = await result.current.wrap({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          amount: 5,
          decimals: 9,
        },
        signTransaction,
      });
    });

    expect(sig).toBe('tx-sig-wrap');
    expect(mockGetSplInterfaceInfos).toHaveBeenCalled();
    expect(signTransaction).toHaveBeenCalledOnce();
    expect(mockRpc.sendRawTransaction).toHaveBeenCalledOnce();
  });

  it('throws when no SPL interface is found', async () => {
    mockGetSplInterfaceInfos.mockResolvedValue([
      { isInitialized: false, tokenProgram: TOKEN_2022_PROGRAM_ID },
    ]);

    const { result } = renderHook(() => useWrap());

    await expect(
      act(async () => {
        await result.current.wrap({
          params: {
            ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
            mint: PublicKey.unique().toBase58(),
            amount: 1,
          },
          signTransaction,
        });
      }),
    ).rejects.toThrow('No SPL interface found');
  });

  it('throws when SPL balance is insufficient', async () => {
    mockGetAccount.mockResolvedValue({ amount: 100n });

    const { result } = renderHook(() => useWrap());

    await expect(
      result.current.wrap({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          amount: 1,
          decimals: 9,
        },
        signTransaction,
      }),
    ).rejects.toThrow();

    expect(mockRpc.sendRawTransaction).not.toHaveBeenCalled();
  });

  it('clears isLoading on error', async () => {
    mockGetSplInterfaceInfos.mockRejectedValue(new Error('RPC fail'));

    const { result } = renderHook(() => useWrap());

    try {
      await act(async () => {
        await result.current.wrap({
          params: {
            ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
            mint: PublicKey.unique().toBase58(),
            amount: 1,
          },
          signTransaction,
        });
      });
    } catch {
      // expected
    }

    expect(result.current.isLoading).toBe(false);
  });
});
