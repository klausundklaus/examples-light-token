import { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useUnifiedBalance } from './hooks/useUnifiedBalance';
import { Header } from './components/ui/Header';
import WalletInfo from './components/sections/WalletInfo';
import TransferForm from './components/sections/TransferForm';
import TransactionStatus from './components/sections/TransactionStatus';
import TransactionHistory from './components/sections/TransactionHistory';

export default function App() {
  const { publicKey, connected } = useWallet();
  const { balances, isLoading: isLoadingBalances, fetchBalances } = useUnifiedBalance();

  const [selectedMint, setSelectedMint] = useState<string>('');
  const [txSignature, setTxSignature] = useState<string | null>(null);
  const [txError, setTxError] = useState<string | null>(null);

  const ownerAddress = publicKey?.toBase58() ?? '';

  useEffect(() => {
    if (!ownerAddress) {
      setSelectedMint('');
      return;
    }

    const loadBalances = async () => {
      await fetchBalances(ownerAddress);
    };

    loadBalances();
  }, [ownerAddress, fetchBalances]);

  useEffect(() => {
    if (balances.length > 0 && !selectedMint) {
      setSelectedMint(balances[0].mint);
    }
  }, [balances, selectedMint]);

  const handleTransferSuccess = async (signature: string) => {
    setTxSignature(signature);
    setTxError(null);
    await fetchBalances(ownerAddress);
  };

  const handleTransferError = (error: string) => {
    setTxError(error);
    setTxSignature(null);
  };

  return (
    <div className="bg-[#E0E7FF66] min-h-screen">
      <Header />
      {connected && publicKey ? (
        <section className="w-full p-8">
          <div className="flex items-center justify-between mb-8">
            <WalletMultiButton />
          </div>

          <div className="flex justify-center">
            <div className="w-full max-w-2xl">
              <WalletInfo address={ownerAddress} />

              <TransferForm
                ownerAddress={ownerAddress}
                selectedMint={selectedMint}
                onMintChange={setSelectedMint}
                balances={balances}
                isLoadingBalances={isLoadingBalances}
                onTransferSuccess={handleTransferSuccess}
                onTransferError={handleTransferError}
              />

              <TransactionStatus signature={txSignature} error={txError} />

              <TransactionHistory ownerAddress={ownerAddress} refreshTrigger={txSignature} />
            </div>
          </div>
        </section>
      ) : (
        <section className="w-full flex flex-col justify-center items-center h-[calc(100vh-60px)] px-4">
          <div className="text-center max-w-md">
            <h1 className="text-3xl md:text-4xl font-semibold text-gray-900 mb-3">
              Send Tokens
            </h1>
            <p className="text-gray-600 mb-8">
              Send light tokens to any Solana address instantly.
            </p>
            <WalletMultiButton />
          </div>
        </section>
      )}
    </div>
  );
}
