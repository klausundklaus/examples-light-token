import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { createMockRpc } from './mock-rpc';

const mockRpc = createMockRpc();

vi.mock('@lightprotocol/stateless.js', () => ({
  createRpc: () => mockRpc,
}));

import { useTransactionHistory } from '../useTransactionHistory';

const OWNER = '7EcDhSYGxXyscszYEp35KHN8vvw3svAuLKTzXwCFLtV';

beforeEach(() => {
  vi.clearAllMocks();
});

describe('useTransactionHistory', () => {
  it('returns empty when address is empty', async () => {
    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory('');
    });

    expect(result.current.transactions).toEqual([]);
    expect(mockRpc.getSignaturesForOwnerInterface).not.toHaveBeenCalled();
  });

  it('fetches and formats transactions', async () => {
    const blockTime = Math.floor(Date.now() / 1000);
    mockRpc.getSignaturesForOwnerInterface.mockResolvedValue({
      signatures: [
        { signature: 'sig1', slot: 100, blockTime },
        { signature: 'sig2', slot: 101, blockTime: blockTime + 60 },
      ],
    });

    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory(OWNER);
    });

    expect(result.current.transactions).toHaveLength(2);
    expect(result.current.transactions[0].signature).toBe('sig1');
    expect(result.current.transactions[0].slot).toBe(100);
    expect(result.current.transactions[0].timestamp).toBe(
      new Date(blockTime * 1000).toISOString(),
    );
  });

  it('respects limit parameter', async () => {
    mockRpc.getSignaturesForOwnerInterface.mockResolvedValue({
      signatures: [
        { signature: 'a', slot: 1, blockTime: 1000 },
        { signature: 'b', slot: 2, blockTime: 2000 },
        { signature: 'c', slot: 3, blockTime: 3000 },
      ],
    });

    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory(OWNER, 2);
    });

    expect(result.current.transactions).toHaveLength(2);
    expect(result.current.transactions[0].signature).toBe('a');
    expect(result.current.transactions[1].signature).toBe('b');
  });

  it('handles null blockTime gracefully', async () => {
    mockRpc.getSignaturesForOwnerInterface.mockResolvedValue({
      signatures: [{ signature: 'sig-null', slot: 50, blockTime: null }],
    });

    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory(OWNER);
    });

    expect(result.current.transactions[0].blockTime).toBe(0);
    expect(result.current.transactions[0].timestamp).toBe('');
  });

  it('sets error state on RPC failure', async () => {
    mockRpc.getSignaturesForOwnerInterface.mockRejectedValue(
      new Error('Network error'),
    );

    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory(OWNER);
    });

    expect(result.current.error).toBe('Network error');
    expect(result.current.transactions).toEqual([]);
  });

  it('clears error on successful refetch', async () => {
    mockRpc.getSignaturesForOwnerInterface.mockRejectedValueOnce(
      new Error('fail'),
    );

    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory(OWNER);
    });
    expect(result.current.error).toBe('fail');

    mockRpc.getSignaturesForOwnerInterface.mockResolvedValueOnce({
      signatures: [{ signature: 's', slot: 1, blockTime: 1000 }],
    });

    await act(async () => {
      await result.current.fetchTransactionHistory(OWNER);
    });
    expect(result.current.error).toBeNull();
    expect(result.current.transactions).toHaveLength(1);
  });

  it('handles empty signatures array', async () => {
    mockRpc.getSignaturesForOwnerInterface.mockResolvedValue({
      signatures: [],
    });

    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory(OWNER);
    });

    expect(result.current.transactions).toEqual([]);
    expect(result.current.error).toBeNull();
  });
});
