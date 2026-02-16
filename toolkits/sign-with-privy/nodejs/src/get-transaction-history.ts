import 'dotenv/config';
import {createRpc} from '@lightprotocol/stateless.js';
import {PublicKey} from '@solana/web3.js';

const getTransactionHistory = async (
  ownerAddress: string,
  limit: number = 10,
) => {
  const connection = createRpc(process.env.HELIUS_RPC_URL!);

  const owner = new PublicKey(ownerAddress);

  // Get Light Token interface signatures
  const result = await connection.getSignaturesForOwnerInterface(owner);

  if (!result.signatures || result.signatures.length === 0) {
    return {
      count: 0,
      transactions: [],
    };
  }

  const limitedSignatures = result.signatures.slice(0, limit);

  const transactions = limitedSignatures.map((sig) => ({
    signature: sig.signature,
    slot: sig.slot,
    blockTime: sig.blockTime ?? 0,
    timestamp: sig.blockTime ? new Date(sig.blockTime * 1000).toISOString() : '',
  }));

  return {
    count: result.signatures.length,
    transactions,
  };
};

export default getTransactionHistory;

// --- main ---
import {TREASURY_WALLET_ADDRESS} from './config.js';
getTransactionHistory(TREASURY_WALLET_ADDRESS)
  .then((result) => console.log('Transaction history:', JSON.stringify(result, null, 2)))
  .catch(console.error);
