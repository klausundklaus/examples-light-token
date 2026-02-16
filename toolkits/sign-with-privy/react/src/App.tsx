import { useState, useEffect } from 'react';
import { usePrivy } from '@privy-io/react-auth';
import { useWallets } from '@privy-io/react-auth/solana';
import { useUnifiedBalance } from './hooks/useUnifiedBalance';
import { Header } from './components/ui/Header';
import WalletInfo from './components/sections/WalletInfo';
import TransferForm from './components/sections/TransferForm';
import TransactionStatus from './components/sections/TransactionStatus';
import TransactionHistory from './components/sections/TransactionHistory';
import { ArrowLeftIcon, ClipboardIcon } from '@heroicons/react/24/outline';

export default function App() {
  const { login, logout, authenticated, user } = usePrivy();
  const { wallets } = useWallets();
  const { balances, isLoading: isLoadingBalances, fetchBalances } = useUnifiedBalance();

  const [selectedWallet, setSelectedWallet] = useState<string>('');
  const [selectedMint, setSelectedMint] = useState<string>('');
  const [txSignature, setTxSignature] = useState<string | null>(null);
  const [txError, setTxError] = useState<string | null>(null);

  useEffect(() => {
    if (wallets.length > 0 && !selectedWallet) {
      setSelectedWallet(wallets[0].address);
    }
  }, [wallets, selectedWallet]);

  useEffect(() => {
    if (!selectedWallet) {
      setSelectedMint('');
      return;
    }

    const loadBalances = async () => {
      await fetchBalances(selectedWallet);
    };

    loadBalances();
  }, [selectedWallet, fetchBalances]);

  useEffect(() => {
    if (balances.length > 0 && !selectedMint) {
      setSelectedMint(balances[0].mint);
    }
  }, [balances, selectedMint]);

  const handleWalletChange = (address: string) => {
    setSelectedWallet(address);
    setSelectedMint('');
  };

  const handleTransferSuccess = async (signature: string) => {
    setTxSignature(signature);
    setTxError(null);
    await fetchBalances(selectedWallet);
  };

  const handleTransferError = (error: string) => {
    setTxError(error);
    setTxSignature(null);
  };

  const copyAddress = () => {
    if (user?.wallet?.address) {
      navigator.clipboard.writeText(user.wallet.address);
      alert('Address copied!');
    }
  };

  return (
    <div className="bg-[#E0E7FF66] min-h-screen">
      <Header />
      {authenticated ? (
        <section className="w-full p-8">
          <div className="flex items-center justify-between mb-8">
            <button className="button" onClick={logout}>
              <ArrowLeftIcon className="h-4 w-4" /> Logout
            </button>
            {user?.wallet?.address && (
              <button
                onClick={copyAddress}
                className="flex items-center gap-2 text-sm text-gray-600 hover:text-gray-900 cursor-pointer px-3 py-1 rounded-lg hover:bg-gray-100 transition-colors"
              >
                <span>{user.wallet.address.slice(0, 8)}...{user.wallet.address.slice(-8)}</span>
                <ClipboardIcon className="h-4 w-4" />
              </button>
            )}
          </div>

          <div className="flex justify-center">
            <div className="w-full max-w-2xl">
              <WalletInfo address={selectedWallet} />

              <TransferForm
                selectedWallet={selectedWallet}
                wallets={wallets}
                onWalletChange={handleWalletChange}
                selectedMint={selectedMint}
                onMintChange={setSelectedMint}
                balances={balances}
                isLoadingBalances={isLoadingBalances}
                onTransferSuccess={handleTransferSuccess}
                onTransferError={handleTransferError}
              />

              <TransactionStatus signature={txSignature} error={txError} />

              <TransactionHistory ownerAddress={selectedWallet} refreshTrigger={txSignature} />
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
            <button
              className="button-primary px-8 py-3"
              onClick={login}
            >
              Get started
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
