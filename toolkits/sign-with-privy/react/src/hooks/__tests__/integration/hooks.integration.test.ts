import { describe, it, expect } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { renderHook, act } from '@testing-library/react';
import { useUnifiedBalance } from '../../useUnifiedBalance';
import { useTransactionHistory } from '../../useTransactionHistory';

const RPC_URL = import.meta.env.VITE_HELIUS_RPC_URL;

// Fresh keypair — no token accounts, minimal RPC calls
const TEST_ADDRESS = Keypair.generate().publicKey.toBase58();

describe.runIf(RPC_URL)('hooks (devnet integration)', () => {
  describe('useUnifiedBalance', () => {
    it('fetches SOL balance for empty wallet', async () => {
      const { result } = renderHook(() => useUnifiedBalance());

      await act(async () => {
        await result.current.fetchBalances(TEST_ADDRESS);
      });

      expect(result.current.isLoading).toBe(false);
      expect(Array.isArray(result.current.balances)).toBe(true);

      const sol = result.current.balances.find((b) => b.isNative);
      expect(sol).toBeDefined();
      expect(sol!.spl).toBe(0n);
      expect(sol!.decimals).toBe(9);
    });
  });

  describe('useTransactionHistory', () => {
    it('returns empty for address with no history', async () => {
      const { result } = renderHook(() => useTransactionHistory());

      await act(async () => {
        await result.current.fetchTransactionHistory(TEST_ADDRESS, 5);
      });

      expect(result.current.isLoading).toBe(false);
      expect(result.current.transactions).toEqual([]);
      expect(result.current.error).toBeNull();
    });
  });
});
