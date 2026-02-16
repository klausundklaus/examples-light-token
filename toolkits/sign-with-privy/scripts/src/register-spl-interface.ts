import 'dotenv/config';
import {Keypair, PublicKey} from '@solana/web3.js';
import {createRpc} from '@lightprotocol/stateless.js';
import {createSplInterface} from '@lightprotocol/compressed-token';
import {homedir} from 'os';
import {readFileSync} from 'fs';

// Add to existing mints an SPL interface PDA to enable interop with Light Tokens.
// Interface PDA holds SPL/T22 tokens when wrapped to light-token.
const registerSplInterface = async (mintAddress: string, tokenProgramId?: PublicKey) => {
  const connection = createRpc(process.env.HELIUS_RPC_URL!);

  // Load filesystem wallet
  const payer = Keypair.fromSecretKey(
    new Uint8Array(
      JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, 'utf8'))
    )
  );

  const mint = new PublicKey(mintAddress);
  const tx = await createSplInterface(connection, payer, mint, undefined, tokenProgramId);

  console.log('Mint:', mint.toBase58());
  console.log('Register SPL interface signature:', tx);

  return tx;
};

export {registerSplInterface};
export default registerSplInterface;

// --- main ---
// Usage: npm run register:spl-interface <mint>
// Falls back to TEST_MINT env var.
const mint = process.argv[2] || process.env.TEST_MINT;

if (!mint) {
  console.error('Usage: npm run register:spl-interface <mint>');
  console.error('  or set TEST_MINT in .env');
  process.exit(1);
}

registerSplInterface(mint)
  .then((result) => console.log('Result:', result))
  .catch(console.error);
