import {useState, useCallback} from 'react';
import {PublicKey} from '@solana/web3.js';
import {createRpc} from '@lightprotocol/stateless.js';
import {
  getAssociatedTokenAddressInterface,
  getAtaInterface,
} from '@lightprotocol/compressed-token/unified';

export interface LightBalance {
  /** Hot balance from Light Token associated token account. */
  hot: bigint;
  /** Cold balance from compressed token accounts. */
  cold: bigint;
  /** Combined hot + cold balance. */
  unified: bigint;
}

export function useLightBalance(mintAddress: string) {
  const [balance, setBalance] = useState<LightBalance>({hot: 0n, cold: 0n, unified: 0n});
  const [isLoading, setIsLoading] = useState(false);

  const fetchBalance = useCallback(
    async (ownerAddress: string) => {
      if (!ownerAddress || !mintAddress) return;

      setIsLoading(true);
      try {
        const rpc = createRpc(import.meta.env.VITE_HELIUS_RPC_URL);
        const owner = new PublicKey(ownerAddress);
        const mint = new PublicKey(mintAddress);

        // Hot balance from Light Token associated token account
        const ata = getAssociatedTokenAddressInterface(mint, owner);
        let hot = 0n;
        try {
          const {parsed} = await getAtaInterface(rpc, ata, owner, mint);
          hot = BigInt(parsed.amount.toString());
        } catch {
          // Associated token account does not exist
        }

        // Cold balance from compressed token accounts
        let cold = 0n;
        try {
          const compressed = await rpc.getCompressedTokenBalancesByOwnerV2(owner, {mint});
          for (const item of compressed.value.items) {
            cold += BigInt(item.balance.toString());
          }
        } catch {
          // No compressed token accounts
        }

        setBalance({hot, cold, unified: hot + cold});
      } catch (error) {
        console.error('Failed to fetch Light Token balance:', error);
        setBalance({hot: 0n, cold: 0n, unified: 0n});
      } finally {
        setIsLoading(false);
      }
    },
    [mintAddress],
  );

  return {balance, isLoading, fetchBalance};
}
