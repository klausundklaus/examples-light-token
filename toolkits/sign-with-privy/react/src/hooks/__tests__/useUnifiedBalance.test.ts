import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { PublicKey } from '@solana/web3.js';
import { createMockRpc, buildTokenAccountData } from './mock-rpc';

const mockRpc = createMockRpc();

vi.mock('@lightprotocol/stateless.js', () => ({
  createRpc: () => mockRpc,
}));

// Deterministic light-token ATA address for mock comparison
const MOCK_LIGHT_ATA = new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');

const mockGetAtaInterface = vi.fn();

vi.mock('@lightprotocol/compressed-token/unified', () => ({
  getAssociatedTokenAddressInterface: () => MOCK_LIGHT_ATA,
  getAtaInterface: (...args: unknown[]) => mockGetAtaInterface(...args),
}));

// Import after mocks are hoisted
import { useUnifiedBalance } from '../useUnifiedBalance';

const OWNER = '7EcDhSYGxXyscszYEp35KHN8vvw3svAuLKTzXwCFLtV';

beforeEach(() => {
  vi.clearAllMocks();
  mockRpc.getBalance.mockResolvedValue(2_500_000_000); // 2.5 SOL
  mockRpc.getTokenAccountsByOwner.mockResolvedValue({ value: [] });
  mockRpc.getCompressedTokenBalancesByOwnerV2.mockResolvedValue({
    value: { items: [] },
  });
  // Default: getAtaInterface throws (no ATA exists)
  mockGetAtaInterface.mockRejectedValue(new Error('Account not found'));
});

describe('useUnifiedBalance', () => {
  it('returns empty balances and does not fetch when address is empty', async () => {
    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances('');
    });

    expect(result.current.balances).toEqual([]);
    expect(mockRpc.getBalance).not.toHaveBeenCalled();
  });

  it('fetches SOL balance as native entry', async () => {
    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances(OWNER);
    });

    const sol = result.current.balances.find((b) => b.isNative);
    expect(sol).toBeDefined();
    expect(sol!.spl).toBe(BigInt(2_500_000_000));
    expect(sol!.decimals).toBe(9);
    expect(sol!.hot).toBe(0n);
    expect(sol!.cold).toBe(0n);
    expect(sol!.t22).toBe(0n);
    expect(sol!.unified).toBe(0n);
  });

  it('parses SPL token account from raw buffer', async () => {
    const mint = PublicKey.unique();
    const data = buildTokenAccountData(mint, 500_000n);

    // First call: SPL accounts, second call: T22 accounts
    mockRpc.getTokenAccountsByOwner
      .mockResolvedValueOnce({
        value: [{ pubkey: PublicKey.unique(), account: { data } }],
      })
      .mockResolvedValueOnce({ value: [] });

    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances(OWNER);
    });

    const spl = result.current.balances.find(
      (b) => b.mint === mint.toBase58(),
    );
    expect(spl).toBeDefined();
    expect(spl!.spl).toBe(500_000n);
    expect(spl!.t22).toBe(0n);
    expect(spl!.hot).toBe(0n);
    expect(spl!.cold).toBe(0n);
    expect(spl!.unified).toBe(0n);
    expect(spl!.isNative).toBe(false);
  });

  it('excludes light-token ATA from T22 balance', async () => {
    const mint = PublicKey.unique();
    const data = buildTokenAccountData(mint, 1_000_000n);

    // SPL: empty, T22: one account whose pubkey matches the mock light ATA
    mockRpc.getTokenAccountsByOwner
      .mockResolvedValueOnce({ value: [] })
      .mockResolvedValueOnce({
        value: [{ pubkey: MOCK_LIGHT_ATA, account: { data } }],
      });

    // getAtaInterface returns the hot balance for this mint
    mockGetAtaInterface.mockResolvedValue({
      parsed: { amount: 1_000_000n },
    });

    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances(OWNER);
    });

    const entry = result.current.balances.find(
      (b) => b.mint === mint.toBase58(),
    );
    expect(entry).toBeDefined();
    // T22 balance should be 0 (light-token ATA excluded)
    expect(entry!.t22).toBe(0n);
    // Hot balance should come from getAtaInterface
    expect(entry!.hot).toBe(1_000_000n);
    expect(entry!.unified).toBe(1_000_000n);
  });

  it('aggregates hot and cold balances into unified', async () => {
    const mint = PublicKey.unique();
    const hotData = buildTokenAccountData(mint, 300_000n);

    // T22 account is a light-token ATA
    mockRpc.getTokenAccountsByOwner
      .mockResolvedValueOnce({ value: [] })
      .mockResolvedValueOnce({
        value: [{ pubkey: MOCK_LIGHT_ATA, account: { data: hotData } }],
      });

    // Hot balance via getAtaInterface: 300k
    mockGetAtaInterface.mockResolvedValue({
      parsed: { amount: 300_000n },
    });

    // Cold balance: 200k for same mint
    mockRpc.getCompressedTokenBalancesByOwnerV2.mockResolvedValue({
      value: {
        items: [{ mint, balance: 200_000n }],
      },
    });

    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances(OWNER);
    });

    const entry = result.current.balances.find(
      (b) => b.mint === mint.toBase58(),
    );
    expect(entry).toBeDefined();
    expect(entry!.hot).toBe(300_000n);
    expect(entry!.cold).toBe(200_000n);
    // unified = hot + cold
    expect(entry!.unified).toBe(500_000n);
    expect(entry!.t22).toBe(0n);
  });

  it('adds standalone cold balance when no hot ATA exists', async () => {
    const coldMint = PublicKey.unique();

    mockRpc.getTokenAccountsByOwner.mockResolvedValue({ value: [] });
    mockRpc.getCompressedTokenBalancesByOwnerV2.mockResolvedValue({
      value: {
        items: [{ mint: coldMint, balance: 750_000n }],
      },
    });

    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances(OWNER);
    });

    const cold = result.current.balances.find(
      (b) => b.mint === coldMint.toBase58(),
    );
    expect(cold).toBeDefined();
    expect(cold!.cold).toBe(750_000n);
    expect(cold!.hot).toBe(0n);
    // unified = hot(0) + cold(750k) = 750k
    expect(cold!.unified).toBe(750_000n);
    expect(cold!.spl).toBe(0n);
    expect(cold!.t22).toBe(0n);
  });

  it('sets isLoading during fetch and clears after', async () => {
    const { result } = renderHook(() => useUnifiedBalance());

    expect(result.current.isLoading).toBe(false);

    let resolveBalance!: (v: number) => void;
    mockRpc.getBalance.mockReturnValue(
      new Promise((r) => {
        resolveBalance = r;
      }),
    );

    const fetchPromise = act(async () => {
      const p = result.current.fetchBalances(OWNER);
      return p;
    });

    resolveBalance(1_000_000_000);
    await fetchPromise;

    expect(result.current.isLoading).toBe(false);
  });

  it('returns empty balances on complete RPC failure', async () => {
    mockRpc.getBalance.mockRejectedValue(new Error('RPC down'));
    mockRpc.getTokenAccountsByOwner.mockRejectedValue(new Error('RPC down'));
    mockRpc.getCompressedTokenBalancesByOwnerV2.mockRejectedValue(
      new Error('RPC down'),
    );

    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances(OWNER);
    });

    expect(result.current.isLoading).toBe(false);
  });
});
