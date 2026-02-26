import { useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { useTransfer } from '../../hooks/useTransfer';
import type { TokenBalance } from '../../hooks/useUnifiedBalance';
import CopyButton from '../reusables/CopyButton';
import Section from '../reusables/Section';

interface TransferFormProps {
  ownerAddress: string;
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
  ownerAddress,
  selectedMint,
  onMintChange,
  balances,
  isLoadingBalances,
  onTransferSuccess,
  onTransferError,
}: TransferFormProps) {
  const { signTransaction } = useWallet();
  const { transfer } = useTransfer();

  const [recipientAddress, setRecipientAddress] = useState<string>('');
  const [amount, setAmount] = useState<string>('1');
  const [isLoading, setIsLoading] = useState(false);

  const handleTransfer = async () => {
    if (!ownerAddress || !selectedMint || !recipientAddress) return;

    const amountNum = parseFloat(amount);
    if (isNaN(amountNum) || amountNum <= 0) {
      alert('Please enter a valid amount');
      return;
    }

    if (!signTransaction) {
      onTransferError('Wallet does not support signTransaction');
      return;
    }

    const selectedToken = balances.find(b => b.mint === selectedMint);
    if (!selectedToken || selectedToken.unified === 0n) return;

    setIsLoading(true);

    try {
      const signature = await transfer({
        params: {
          ownerPublicKey: ownerAddress,
          mint: selectedMint,
          toAddress: recipientAddress,
          amount: amountNum,
          decimals: selectedToken.decimals,
        },
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
        <label htmlFor="wallet-display" className="block text-sm font-medium mb-2">
          From wallet
        </label>
        <div className="flex gap-2">
          <input
            id="wallet-display"
            type="text"
            value={ownerAddress ? `${ownerAddress.slice(0, 8)}...${ownerAddress.slice(-8)}` : ''}
            className="input flex-1"
            disabled
          />
          {ownerAddress && (
            <CopyButton text={ownerAddress} label="Address" />
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
