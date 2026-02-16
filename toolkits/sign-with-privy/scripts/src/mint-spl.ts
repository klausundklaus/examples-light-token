import 'dotenv/config';
import {Keypair, PublicKey, Transaction, ComputeBudgetProgram, sendAndConfirmTransaction} from '@solana/web3.js';
import {createRpc} from '@lightprotocol/stateless.js';
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAccount,
  TokenAccountNotFoundError,
} from '@solana/spl-token';
import {homedir} from 'os';
import {readFileSync} from 'fs';

const mintSplTokens = async (
  mintAddress: string,
  recipientAddress: string,
  amount: number,
  decimals: number,
) => {
  const connection = createRpc(process.env.HELIUS_RPC_URL!);

  // Load filesystem wallet
  const payer = Keypair.fromSecretKey(
    new Uint8Array(
      JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, 'utf8'))
    )
  );

  const mint = new PublicKey(mintAddress);
  const recipient = new PublicKey(recipientAddress);
  const tokenAmount = BigInt(Math.floor(amount * Math.pow(10, decimals)));

  // Auto-detect token program (SPL vs T22) from mint account owner
  const mintAccountInfo = await connection.getAccountInfo(mint);
  if (!mintAccountInfo) throw new Error(`Mint account ${mintAddress} not found`);
  const tokenProgramId = mintAccountInfo.owner;
  console.log('Detected token program:', tokenProgramId.toBase58());

  // Derive recipient associated token account for the given token program (SPL or T22)
  const recipientAta = getAssociatedTokenAddressSync(mint, recipient, false, tokenProgramId);

  // Build transaction
  const transaction = new Transaction();
  transaction.add(ComputeBudgetProgram.setComputeUnitLimit({units: 300_000}));

  // Create associated token account if it doesn't exist
  try {
    await getAccount(connection, recipientAta, undefined, tokenProgramId);
  } catch (e) {
    if (e instanceof TokenAccountNotFoundError) {
      transaction.add(
        createAssociatedTokenAccountInstruction(payer.publicKey, recipientAta, recipient, mint, tokenProgramId)
      );
    } else {
      throw e;
    }
  }

  // Add mint-to instruction
  transaction.add(
    createMintToInstruction(
      mint,
      recipientAta,
      payer.publicKey,
      tokenAmount,
      [],
      tokenProgramId,
    )
  );

  // Send transaction
  const signature = await sendAndConfirmTransaction(
    connection,
    transaction,
    [payer],
    {commitment: 'confirmed'}
  );

  return signature;
};

export default mintSplTokens;

// --- main ---
// Usage: npm run mint:spl <mint> <recipient> [amount] [decimals]
// Falls back to TEST_MINT and RECIPIENT_ADDRESS env vars.
const mint = process.argv[2] || process.env.TEST_MINT;
const recipient = process.argv[3] || process.env.RECIPIENT_ADDRESS;
const amount = Number(process.argv[4] || 100);
const decimals = Number(process.argv[5] || 9);

if (!mint || !recipient) {
  console.error('Usage: npm run mint:spl <mint> <recipient> [amount] [decimals]');
  console.error('  or set TEST_MINT and RECIPIENT_ADDRESS in .env');
  process.exit(1);
}

mintSplTokens(mint, recipient, amount, decimals)
  .then((result) => console.log('Mint SPL signature:', result))
  .catch(console.error);
