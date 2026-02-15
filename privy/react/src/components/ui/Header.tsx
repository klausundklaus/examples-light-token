export function Header() {
  return (
    <header className="w-full px-8 py-4 bg-white border-b border-gray-200">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-lg font-semibold text-gray-900">Light Token</span>
          <span className="text-sm text-gray-500">+ Privy</span>
        </div>
        <div className="flex items-center gap-4">
          <a
            href="https://www.zkcompression.com/light-token/welcome"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-gray-600 hover:text-indigo-600 transition-colors"
          >
            Light Token Docs
          </a>
          <a
            href="https://docs.privy.io/guide/react/wallets/usage/signing"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-gray-600 hover:text-indigo-600 transition-colors"
          >
            Privy Docs
          </a>
        </div>
      </div>
    </header>
  );
}
