import { useState } from 'react';
import type { ConnectedStandardSolanaWallet } from '@privy-io/js-sdk-core';
import { useSignTransaction } from '@privy-io/react-auth/solana';
import { useTransfer } from '../../hooks/useTransfer';
import type { TokenBalance } from '../../hooks/useUnifiedBalance';
import CopyButton from '../reusables/CopyButton';
import Section from '../reusables/Section';

interface TransferFormProps {
  selectedWallet: string;
  wallets: ConnectedStandardSolanaWallet[];
  onWalletChange: (address: string) => void;
  selectedMint: string;
  onMintChange: (mint: string) => void;
  balances: TokenBalance[];
  isLoadingBalances: boolean;
  onTransferSuccess: (signature: string) => void;
  onTransferError: (error: string) => void;
}

function formatBigint(value: bigint, decimals: number): string {
  const divisor = BigInt(10 ** decimals);
  const whole = value / divisor;
  const fraction = value % divisor;
  const fractionStr = fraction.toString().padStart(decimals, '0').replace(/0+$/, '');
  return fractionStr ? `${whole}.${fractionStr}` : whole.toString();
}

function totalBalance(balance: TokenBalance): bigint {
  if (balance.isNative) return balance.spl;
  return balance.unified + balance.spl + balance.t22;
}

export default function TransferForm({
  selectedWallet,
  wallets,
  onWalletChange,
  selectedMint,
  onMintChange,
  balances,
  isLoadingBalances,
  onTransferSuccess,
  onTransferError,
}: TransferFormProps) {
  const { signTransaction } = useSignTransaction();
  const { transfer } = useTransfer();

  const [recipientAddress, setRecipientAddress] = useState<string>('');
  const [amount, setAmount] = useState<string>('1');
  const [isLoading, setIsLoading] = useState(false);

  const handleTransfer = async () => {
    if (!selectedWallet || !selectedMint || !recipientAddress) return;

    const amountNum = parseFloat(amount);
    if (isNaN(amountNum) || amountNum <= 0) {
      alert('Please enter a valid amount');
      return;
    }

    const wallet = wallets.find((w) => w.address === selectedWallet);
    if (!wallet) return;

    const selectedToken = balances.find(b => b.mint === selectedMint);
    if (!selectedToken || selectedToken.unified === 0n) return;

    setIsLoading(true);

    try {
      const signature = await transfer({
        params: {
          ownerPublicKey: selectedWallet,
          mint: selectedMint,
          toAddress: recipientAddress,
          amount: amountNum,
          decimals: selectedToken.decimals,
        },
        wallet,
        signTransaction,
      });

      setRecipientAddress('');
      setAmount('1');
      onTransferSuccess(signature);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error('Transfer error:', error);
      onTransferError(message);
    } finally {
      setIsLoading(false);
    }
  };

  const selectedBalance = balances.find(b => b.mint === selectedMint);
  const canSend = selectedBalance && !selectedBalance.isNative
    && selectedBalance.unified > 0n;

  return (
    <Section name="Send Tokens">
      <div className="mb-4">
        <label htmlFor="wallet-select" className="block text-sm font-medium mb-2">
          From wallet
        </label>
        <div className="flex gap-2">
          <select
            id="wallet-select"
            value={selectedWallet}
            onChange={(e) => onWalletChange(e.target.value)}
            className="input flex-1"
            disabled={wallets.length === 0}
          >
            {wallets.length === 0 ? (
              <option value="">No wallets available</option>
            ) : (
              wallets.map((wallet) => (
                <option key={wallet.address} value={wallet.address}>
                  {wallet.address.slice(0, 8)}...{wallet.address.slice(-8)}
                </option>
              ))
            )}
          </select>
          {selectedWallet && (
            <CopyButton text={selectedWallet} label="Address" />
          )}
        </div>
      </div>

      <div className="mb-4">
        <label htmlFor="mint-select" className="block text-sm font-medium mb-2">
          Token
        </label>
        <div className="flex gap-2">
          <select
            id="mint-select"
            value={selectedMint}
            onChange={(e) => onMintChange(e.target.value)}
            className="input flex-1"
            disabled={isLoadingBalances || balances.length === 0}
          >
            {isLoadingBalances ? (
              <option value="">Loading tokens...</option>
            ) : balances.length === 0 ? (
              <option value="">No tokens found</option>
            ) : (
              balances.map((balance) => {
                const total = totalBalance(balance);
                const formattedAmount = formatBigint(total, balance.decimals);

                const label = balance.isNative
                  ? `SOL - ${formattedAmount}`
                  : `${balance.mint.slice(0, 8)}...${balance.mint.slice(-4)} - ${formattedAmount}`;

                return (
                  <option key={balance.mint} value={balance.mint}>
                    {label}
                  </option>
                );
              })
            )}
          </select>
        </div>
      </div>

      <div className="mb-4">
        <label htmlFor="recipient" className="block text-sm font-medium mb-2">
          Recipient address
        </label>
        <input
          id="recipient"
          type="text"
          value={recipientAddress}
          onChange={(e) => setRecipientAddress(e.target.value)}
          placeholder="Enter Solana address"
          className="input"
        />
      </div>

      <div className="mb-6">
        <label htmlFor="amount" className="block text-sm font-medium mb-2">
          Amount
        </label>
        <input
          id="amount"
          type="number"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          placeholder="1"
          min="0"
          step="0.000001"
          className="input"
        />
      </div>

      <button
        onClick={handleTransfer}
        disabled={isLoading || !canSend || !recipientAddress}
        className="button-primary w-full"
      >
        {isLoading ? 'Sending...' : 'Send'}
      </button>
    </Section>
  );
}
