export const PRIVY_APP_ID = process.env.PRIVY_APP_ID!;
export const PRIVY_APP_SECRET = process.env.PRIVY_APP_SECRET!;

export const TREASURY_WALLET_ID = process.env.TREASURY_WALLET_ID!;
export const TREASURY_WALLET_ADDRESS = process.env.TREASURY_WALLET_ADDRESS!;
export const TREASURY_AUTHORIZATION_KEY = process.env.TREASURY_AUTHORIZATION_KEY!;

export const HELIUS_RPC_URL = process.env.HELIUS_RPC_URL!;
export const TEST_MINT = process.env.TEST_MINT || '';

export const SOLANA_CAIP2_DEVNET = 'solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1';

// Default values for app scripts
export const DEFAULT_TEST_RECIPIENT = process.env.DEFAULT_TEST_RECIPIENT || TREASURY_WALLET_ADDRESS;
export const DEFAULT_AMOUNT = process.env.DEFAULT_AMOUNT || '0.001';
export const DEFAULT_DECIMALS = process.env.DEFAULT_DECIMALS || '9';

// Validate required env vars
if (!PRIVY_APP_ID || !PRIVY_APP_SECRET) {
  throw new Error('Missing Privy credentials (PRIVY_APP_ID, PRIVY_APP_SECRET)');
}

if (!TREASURY_WALLET_ID || !TREASURY_WALLET_ADDRESS || !TREASURY_AUTHORIZATION_KEY) {
  throw new Error(
    'Missing treasury wallet configuration. Please add TREASURY_AUTHORIZATION_KEY to .env'
  );
}

if (!HELIUS_RPC_URL) {
  throw new Error('Missing HELIUS_RPC_URL');
}
