import 'dotenv/config';
import {PublicKey} from '@solana/web3.js';
import {createRpc} from '@lightprotocol/stateless.js';
import {
  getAssociatedTokenAddressInterface,
  getAtaInterface,
} from '@lightprotocol/compressed-token/unified';

export async function getLightBalance(
  ownerAddress: string,
  mintAddress: string,
): Promise<{hot: number; cold: number; unified: number}> {
  const rpc = createRpc(process.env.HELIUS_RPC_URL!);
  const owner = new PublicKey(ownerAddress);
  const mint = new PublicKey(mintAddress);

  // Hot balance from Light Token associated token account
  const ata = getAssociatedTokenAddressInterface(mint, owner);
  let hot = 0;
  try {
    const {parsed} = await getAtaInterface(rpc, ata, owner, mint);
    hot = Number(parsed.amount) / 1e9;
  } catch {
    // Associated token account does not exist
  }

  // Cold balance from compressed token accounts
  let cold = 0;
  try {
    const compressed = await rpc.getCompressedTokenBalancesByOwnerV2(owner, {mint});
    for (const item of compressed.value.items) {
      cold += Number(BigInt(item.balance.toString())) / 1e9;
    }
  } catch {
    // No compressed token accounts
  }

  return {hot, cold, unified: hot + cold};
}

export default getLightBalance;

// --- main ---
import {TREASURY_WALLET_ADDRESS, TEST_MINT} from './config.js';

if (!TEST_MINT) {
  console.error('Set TEST_MINT in .env');
  process.exit(1);
}

getLightBalance(TREASURY_WALLET_ADDRESS, TEST_MINT)
  .then((b) => {
    console.log(`Hot (Light Token associated token account): ${b.hot}`);
    console.log(`Cold (compressed token accounts):           ${b.cold}`);
    console.log(`Unified:                                    ${b.unified}`);
  })
  .catch(console.error);