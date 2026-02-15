import { useEffect, useState } from 'react';
import { useTransactionHistory } from '../../hooks/useTransactionHistory';
import Section from '../reusables/Section';
import CopyButton from '../reusables/CopyButton';

interface TransactionHistoryProps {
  ownerAddress: string;
  refreshTrigger?: string | null;
}

export default function TransactionHistory({ ownerAddress, refreshTrigger }: TransactionHistoryProps) {
  const { transactions, isLoading, error, fetchTransactionHistory } = useTransactionHistory();
  const [isExpanded, setIsExpanded] = useState(false);

  useEffect(() => {
    if (ownerAddress) {
      fetchTransactionHistory(ownerAddress);
    }
  }, [ownerAddress, refreshTrigger, fetchTransactionHistory]);

  if (!ownerAddress) return null;

  return (
    <Section name="Transaction History">
      {isLoading && <p className="text-sm text-gray-500">Loading...</p>}
      {error && <p className="text-sm text-red-600">{error}</p>}
      {!isLoading && transactions.length === 0 && !error && (
        <p className="text-sm text-gray-500">No transactions found.</p>
      )}
      {transactions.length > 0 && (
        <div className="space-y-2">
          {transactions.slice(0, isExpanded ? undefined : 5).map((tx) => (
            <div
              key={tx.signature}
              className="flex items-center justify-between py-2 border-b border-gray-100 last:border-0"
            >
              <div>
                <p className="font-mono text-xs text-gray-700">
                  {tx.signature.slice(0, 16)}...{tx.signature.slice(-8)}
                </p>
                {tx.timestamp && (
                  <p className="text-xs text-gray-400">
                    {new Date(tx.timestamp).toLocaleString()}
                  </p>
                )}
              </div>
              <div className="flex items-center gap-2">
                <CopyButton text={tx.signature} label="Signature" />
                <a
                  href={`https://explorer.solana.com/tx/${tx.signature}?cluster=devnet`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-indigo-600 hover:text-indigo-800"
                >
                  Explorer
                </a>
              </div>
            </div>
          ))}
          {transactions.length > 5 && (
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="text-xs text-indigo-600 hover:text-indigo-800"
            >
              {isExpanded ? 'Show less' : `Show all (${transactions.length})`}
            </button>
          )}
        </div>
      )}
    </Section>
  );
}
