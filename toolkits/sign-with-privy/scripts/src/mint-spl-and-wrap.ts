import 'dotenv/config';
import {Keypair, PublicKey} from '@solana/web3.js';
import {createRpc} from '@lightprotocol/stateless.js';
import {
  createMintInterface,
  createAtaInterfaceIdempotent,
  getAssociatedTokenAddressInterface,
  wrap,
  transferInterface,
} from '@lightprotocol/compressed-token';
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccount,
  mintTo,
} from '@solana/spl-token';
import {homedir} from 'os';
import {readFileSync} from 'fs';

const createLightTokenMint = async (
  decimals: number,
  initialAmount: number,
  recipientAddress: string,
  tokenProgramId: PublicKey = TOKEN_2022_PROGRAM_ID,
) => {
  const connection = createRpc(process.env.HELIUS_RPC_URL!);

  // Load filesystem wallet
  const payer = Keypair.fromSecretKey(
    new Uint8Array(
      JSON.parse(readFileSync(`${homedir()}/.config/solana/id.json`, 'utf8'))
    )
  );

  // Creates on-chain SPL or T22 mint and registers SPL interface PDA in one transaction.
  // SPL interface PDA enables wrap/unwrap between SPL/T22 and light-token.
  const mintKeypair = Keypair.generate();
  const { mint, transactionSignature } = await createMintInterface(
    connection,
    payer,
    payer,
    null,
    decimals,
    mintKeypair,
    undefined,
    tokenProgramId,
  );

  console.log('Mint:', mint.toBase58());
  console.log('Create signature:', transactionSignature);

  if (initialAmount > 0) {
    const recipient = new PublicKey(recipientAddress);
    const tokenAmount = BigInt(Math.floor(initialAmount * Math.pow(10, decimals)));

    // 1. Mint SPL/T22 tokens to payer's associated token account (payer can sign)
    const payerSplAta = await createAssociatedTokenAccount(
      connection,
      payer,
      mint,
      payer.publicKey,
      undefined,
      tokenProgramId,
    );
    await mintTo(
      connection,
      payer,
      mint,
      payerSplAta,
      payer,
      tokenAmount,
      [],
      undefined,
      tokenProgramId,
    );
    console.log('Minted', initialAmount, 'tokens');

    // 2. Wrap SPL/T22 → payer's light associated token account
    await createAtaInterfaceIdempotent(connection, payer, mint, payer.publicKey);
    const payerLightAta = getAssociatedTokenAddressInterface(mint, payer.publicKey);
    await wrap(connection, payer, payerSplAta, payerLightAta, payer, mint, tokenAmount);
    console.log('Wrapped into payer light associated token account');

    // 3. Transfer payer's light associated token account → recipient's light associated token account
    await createAtaInterfaceIdempotent(connection, payer, mint, recipient);
    const recipientLightAta = getAssociatedTokenAddressInterface(mint, recipient);
    await transferInterface(
      connection,
      payer,
      payerLightAta,
      mint,
      recipientLightAta,
      payer,
      tokenAmount,
    );
    console.log('Transferred', initialAmount, 'tokens to', recipientAddress);
  }

  return {
    mintAddress: mint.toBase58(),
    signature: transactionSignature,
  };
};

export default createLightTokenMint;

// --- main ---
// Usage: npm run mint:spl-and-wrap [recipient] [amount] [decimals]
//        npm run mint:spl-and-wrap:spl [recipient] [amount] [decimals]
// Falls back to RECIPIENT_ADDRESS env var.
// Pass --spl to create an SPL mint instead of T22.
const useSpl = process.argv.includes('--spl');
const positionalArgs = process.argv.slice(2).filter(a => a !== '--spl');
const recipient = positionalArgs[0] || process.env.RECIPIENT_ADDRESS;
const amount = Number(positionalArgs[1] || 100);
const decimals = Number(positionalArgs[2] || 9);
const tokenProgramId = useSpl ? TOKEN_PROGRAM_ID : TOKEN_2022_PROGRAM_ID;

if (!recipient) {
  console.error('Usage: npm run mint:spl-and-wrap <recipient> [amount] [decimals]');
  console.error('       npm run mint:spl-and-wrap:spl <recipient> [amount] [decimals]');
  console.error('  or set RECIPIENT_ADDRESS in .env');
  process.exit(1);
}

console.log('Token program:', useSpl ? 'SPL' : 'T22');
createLightTokenMint(decimals, amount, recipient, tokenProgramId)
  .then((result) => console.log('Mint result:', result))
  .catch(console.error);
