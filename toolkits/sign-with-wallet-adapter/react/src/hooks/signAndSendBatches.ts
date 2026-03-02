import { Transaction, TransactionInstruction, PublicKey } from '@solana/web3.js';

export type SignTransactionFn = (transaction: Transaction) => Promise<Transaction>;

interface SignAndSendOptions {
  rpc: any;
  feePayer: PublicKey;
  signTransaction: SignTransactionFn;
}

export async function signAndSendBatches(
  instructionBatches: TransactionInstruction[][],
  options: SignAndSendOptions,
): Promise<string | null> {
  const { rpc, feePayer, signTransaction } = options;
  const signatures: string[] = [];

  for (const ixs of instructionBatches) {
    const tx = new Transaction().add(...ixs);
    const { blockhash } = await rpc.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = feePayer;

    const signedTx = await signTransaction(tx);
    const sig = await rpc.sendRawTransaction(signedTx.serialize(), {
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    });
    await rpc.confirmTransaction(sig, 'confirmed');
    signatures.push(sig);
  }

  return signatures.length > 0 ? signatures[signatures.length - 1] : null;
}
