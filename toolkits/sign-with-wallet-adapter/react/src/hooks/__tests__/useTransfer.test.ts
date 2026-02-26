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

const mockCreateTransferInterfaceInstructions = vi.fn();

vi.mock('@lightprotocol/compressed-token/unified', () => ({
  createTransferInterfaceInstructions: (...args: unknown[]) =>
    mockCreateTransferInterfaceInstructions(...args),
}));

import { useTransfer } from '../useTransfer';

const signTransaction = createMockSignTransaction();

beforeEach(() => {
  vi.clearAllMocks();
  mockRpc.getLatestBlockhash.mockResolvedValue({
    blockhash: '11111111111111111111111111111111',
    lastValidBlockHeight: 100,
  });
  mockRpc.sendRawTransaction.mockResolvedValue('tx-sig-transfer');
  signTransaction.mockImplementation(async (tx: any) => {
    tx.partialSign(TEST_KEYPAIR);
    return tx;
  });
  // Default: single batch with one instruction
  mockCreateTransferInterfaceInstructions.mockResolvedValue([[dummyIx]]);
});

describe('useTransfer', () => {
  it('builds, signs, and sends a transfer transaction', async () => {
    const { result } = renderHook(() => useTransfer());

    let sig: string | undefined;
    await act(async () => {
      sig = await result.current.transfer({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          toAddress: PublicKey.unique().toBase58(),
          amount: 1.5,
          decimals: 9,
        },
        signTransaction,
      });
    });

    expect(sig).toBe('tx-sig-transfer');
    expect(mockCreateTransferInterfaceInstructions).toHaveBeenCalledOnce();
    expect(signTransaction).toHaveBeenCalledOnce();
    expect(mockRpc.sendRawTransaction).toHaveBeenCalledOnce();
  });

  it('handles multiple instruction batches', async () => {
    mockCreateTransferInterfaceInstructions.mockResolvedValue([
      [dummyIx],
      [dummyIx],
    ]);

    const { result } = renderHook(() => useTransfer());

    await act(async () => {
      await result.current.transfer({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          toAddress: PublicKey.unique().toBase58(),
          amount: 1,
        },
        signTransaction,
      });
    });

    expect(signTransaction).toHaveBeenCalledTimes(2);
    expect(mockRpc.sendRawTransaction).toHaveBeenCalledTimes(2);
  });

  it('manages isLoading state', async () => {
    const { result } = renderHook(() => useTransfer());
    expect(result.current.isLoading).toBe(false);

    await act(async () => {
      await result.current.transfer({
        params: {
          ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
          mint: PublicKey.unique().toBase58(),
          toAddress: PublicKey.unique().toBase58(),
          amount: 1,
        },
        signTransaction,
      });
    });

    expect(result.current.isLoading).toBe(false);
  });

  it('propagates RPC errors', async () => {
    mockRpc.sendRawTransaction.mockRejectedValue(
      new Error('Transaction failed'),
    );

    const { result } = renderHook(() => useTransfer());

    await expect(
      act(async () => {
        await result.current.transfer({
          params: {
            ownerPublicKey: TEST_KEYPAIR.publicKey.toBase58(),
            mint: PublicKey.unique().toBase58(),
            toAddress: PublicKey.unique().toBase58(),
            amount: 1,
          },
          signTransaction,
        });
      }),
    ).rejects.toThrow('Transaction failed');

    expect(result.current.isLoading).toBe(false);
  });
});
