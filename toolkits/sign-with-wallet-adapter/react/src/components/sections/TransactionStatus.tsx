import Section from '../reusables/Section';
import CopyButton from '../reusables/CopyButton';

interface TransactionStatusProps {
  signature: string | null;
  error: string | null;
}

export default function TransactionStatus({ signature, error }: TransactionStatusProps) {
  if (!signature && !error) return null;

  return (
    <Section name="Transaction Result">
      {signature && (
        <div className="p-4 bg-green-50 border border-green-200 rounded-lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-green-800">Success!</p>
              <p className="text-xs text-green-600 mt-1 font-mono">
                {signature.slice(0, 20)}...{signature.slice(-20)}
              </p>
            </div>
            <div className="flex gap-2">
              <CopyButton text={signature} label="Signature" />
              <a
                href={`https://explorer.solana.com/tx/${signature}?cluster=devnet`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-green-700 hover:text-green-900 underline"
              >
                View on Explorer
              </a>
            </div>
          </div>
        </div>
      )}
      {error && (
        <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-sm font-medium text-red-800">Error</p>
          <p className="text-xs text-red-600 mt-1">{error}</p>
        </div>
      )}
    </Section>
  );
}
