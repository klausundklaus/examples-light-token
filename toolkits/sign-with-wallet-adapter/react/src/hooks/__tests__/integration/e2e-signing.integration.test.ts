/**
 * E2E integration test for signing hooks.
 *
 * Uses the filesystem keypair (~/.config/solana/id.json) to sign transactions.
 *
 * Modes:
 *   Devnet:   VITE_HELIUS_RPC_URL=<url> pnpm test:integration
 *   Localnet: VITE_LOCALNET=true pnpm test:integration
 *             (requires `light test-validator` running on ports 8899/8784/3001)
 */
import { describe, it, expect, beforeAll } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { Keypair, PublicKey, Transaction } from '@solana/web3.js';
import { readFileSync } from 'fs';
import { homedir } from 'os';
import { createRpc } from '@lightprotocol/stateless.js';
import {
  createMintInterface,
  createAtaInterfaceIdempotent,
  getAssociatedTokenAddressInterface,
  wrap,
} from '@lightprotocol/compressed-token';
import {
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccount,
  mintTo,
} from '@solana/spl-token';
import { useTransfer } from '../../useTransfer';
import { useUnifiedBalance } from '../../useUnifiedBalance';
import { useTransactionHistory } from '../../useTransactionHistory';
import type { SignTransactionFn } from '../../signAndSendBatches';

const RPC_URL = import.meta.env.VITE_HELIUS_RPC_URL;
const IS_LOCALNET = import.meta.env.VITE_LOCALNET === 'true';
const ENABLED = !!RPC_URL || IS_LOCALNET;

// Load filesystem keypair for signing
function loadKeypair(): Keypair {
  const raw = readFileSync(`${homedir()}/.config/solana/id.json`, 'utf8');
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(raw)));
}

// Create a signTransaction function from a Keypair (same interface as wallet-adapter)
function createKeypairSigner(keypair: Keypair): SignTransactionFn {
  return async (tx: Transaction): Promise<Transaction> => {
    tx.partialSign(keypair);
    return tx;
  };
}

describe.runIf(ENABLED)(`e2e signing (${IS_LOCALNET ? 'localnet' : 'devnet'})`, () => {
  let payer: Keypair;
  let signTransaction: SignTransactionFn;
  let rpc: ReturnType<typeof createRpc>;
  let testMint: PublicKey;

  beforeAll(async () => {
    payer = loadKeypair();
    signTransaction = createKeypairSigner(payer);

    // Localnet: createRpc() with no args → uses correct localhost defaults
    // (RPC 8899, Photon 8784, Prover 3001).
    // Devnet: createRpc(url) → Helius bundles all services on one URL.
    rpc = IS_LOCALNET ? createRpc() : createRpc(RPC_URL);

    console.log('Payer:', payer.publicKey.toBase58());

    // On localnet, airdrop SOL; on devnet, just check existing balance
    if (IS_LOCALNET) {
      const sig = await rpc.requestAirdrop(payer.publicKey, 2e9);
      await rpc.confirmTransaction(sig, 'confirmed');
      console.log('Airdropped 2 SOL');
    }

    const balance = await rpc.getBalance(payer.publicKey);
    console.log('SOL balance:', balance / 1e9);
    expect(balance).toBeGreaterThan(0.1e9); // Need at least 0.1 SOL

    // Create a test mint with SPL interface + mint + wrap tokens to payer
    const mintKeypair = Keypair.generate();
    const decimals = 9;
    const amount = 10; // 10 tokens

    console.log('Creating test mint...');
    const { mint } = await createMintInterface(
      rpc,
      payer,
      payer,
      null,
      decimals,
      mintKeypair,
      undefined,
      TOKEN_2022_PROGRAM_ID,
    );
    testMint = mint;
    console.log('Test mint:', testMint.toBase58());

    // Mint SPL tokens to payer
    const tokenAmount = BigInt(amount * 10 ** decimals);
    const payerSplAta = await createAssociatedTokenAccount(
      rpc,
      payer,
      testMint,
      payer.publicKey,
      undefined,
      TOKEN_2022_PROGRAM_ID,
    );
    await mintTo(
      rpc,
      payer,
      testMint,
      payerSplAta,
      payer,
      tokenAmount,
      [],
      undefined,
      TOKEN_2022_PROGRAM_ID,
    );
    console.log('Minted', amount, 'tokens');

    // Wrap into light-token
    await createAtaInterfaceIdempotent(rpc, payer, testMint, payer.publicKey);
    const payerLightAta = getAssociatedTokenAddressInterface(testMint, payer.publicKey);
    await wrap(rpc, payer, payerSplAta, payerLightAta, payer, testMint, tokenAmount);
    console.log('Wrapped into light-token ATA');
  }, 120_000);

  it('useUnifiedBalance returns light-token balance for funded wallet', async () => {
    const { result } = renderHook(() => useUnifiedBalance());

    await act(async () => {
      await result.current.fetchBalances(payer.publicKey.toBase58());
    });

    expect(result.current.isLoading).toBe(false);

    // Should have SOL
    const sol = result.current.balances.find(b => b.isNative);
    expect(sol).toBeDefined();
    expect(sol!.spl).toBeGreaterThan(0n);

    // Should have our test mint with hot balance
    const token = result.current.balances.find(b => b.mint === testMint.toBase58());
    expect(token).toBeDefined();
    expect(token!.hot).toBeGreaterThan(0n);
    expect(token!.unified).toBeGreaterThan(0n);
    console.log('Balance check passed. Hot:', token!.hot.toString(), 'Unified:', token!.unified.toString());
  }, 30_000);

  it('useTransfer sends light-tokens to a fresh address', async () => {
    const recipient = Keypair.generate();
    const { result } = renderHook(() => useTransfer());

    let signature: string | undefined;
    await act(async () => {
      signature = await result.current.transfer({
        params: {
          ownerPublicKey: payer.publicKey.toBase58(),
          mint: testMint.toBase58(),
          toAddress: recipient.publicKey.toBase58(),
          amount: 1,
          decimals: 9,
        },
        signTransaction,
      });
    });

    expect(signature).toBeDefined();
    expect(typeof signature).toBe('string');
    expect(signature!.length).toBeGreaterThan(40);
    console.log('Transfer signature:', signature);
  }, 60_000);

  it('useTransactionHistory returns recent transactions', async () => {
    const { result } = renderHook(() => useTransactionHistory());

    await act(async () => {
      await result.current.fetchTransactionHistory(payer.publicKey.toBase58(), 5);
    });

    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
    // Should have at least the transfer we just did
    expect(result.current.transactions.length).toBeGreaterThan(0);
    console.log('Transaction history:', result.current.transactions.length, 'entries');
  }, 30_000);
});
