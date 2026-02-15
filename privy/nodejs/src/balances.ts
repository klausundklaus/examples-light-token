import 'dotenv/config';
import {PublicKey, LAMPORTS_PER_SOL} from '@solana/web3.js';
import {TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID} from '@solana/spl-token';
import {createRpc} from '@lightprotocol/stateless.js';
import {
  getAtaInterface,
  getAssociatedTokenAddressInterface,
} from '@lightprotocol/compressed-token/unified';

interface TokenBalance {
  mint: string;
  decimals: number;
  hot: number;
  cold: number;
  spl: number;
  t22: number;
  unified: number;
}

interface BalanceBreakdown {
  sol: number;
  tokens: TokenBalance[];
}

export async function getBalances(
  ownerAddress: string,
): Promise<BalanceBreakdown> {
  const rpc = createRpc(process.env.HELIUS_RPC_URL!);
  const owner = new PublicKey(ownerAddress);

  // SOL balance
  let solLamports = 0;
  try {
    solLamports = await rpc.getBalance(owner);
  } catch (e) {
    console.error('Failed to fetch SOL balance:', e);
  }

  // Per-mint accumulator
  const mintMap = new Map<string, {spl: number; t22: number; hot: number; cold: number; decimals: number}>();

  const getOrCreate = (mintStr: string) => {
    let entry = mintMap.get(mintStr);
    if (!entry) {
      entry = {spl: 0, t22: 0, hot: 0, cold: 0, decimals: 9};
      mintMap.set(mintStr, entry);
    }
    return entry;
  };

  // Track Light Token associated token accounts so we don't double-count them as Token 2022 balances
  const lightAtaMints = new Set<string>();

  // 1. SPL accounts (standard token program)
  try {
    const splAccounts = await rpc.getTokenAccountsByOwner(owner, {
      programId: TOKEN_PROGRAM_ID,
    });
    for (const {account} of splAccounts.value) {
      const buf = toBuffer(account.data);
      if (!buf || buf.length < 72) continue;
      const mint = new PublicKey(buf.subarray(0, 32));
      const amount = buf.readBigUInt64LE(64);
      const mintStr = mint.toBase58();
      getOrCreate(mintStr).spl += toUiAmount(amount, 9);
    }
  } catch {
    // No SPL accounts
  }

  // 2. T22 accounts (Token-2022) — skip Light Token associated token accounts
  try {
    const t22Accounts = await rpc.getTokenAccountsByOwner(owner, {
      programId: TOKEN_2022_PROGRAM_ID,
    });
    for (const {pubkey, account} of t22Accounts.value) {
      const buf = toBuffer(account.data);
      if (!buf || buf.length < 72) continue;
      const mint = new PublicKey(buf.subarray(0, 32));
      const mintStr = mint.toBase58();
      const expectedAta = getAssociatedTokenAddressInterface(mint, owner);
      if (pubkey.equals(expectedAta)) {
        // Light Token associated token account — query via getAtaInterface instead
        lightAtaMints.add(mintStr);
        getOrCreate(mintStr);
      } else {
        const amount = buf.readBigUInt64LE(64);
        getOrCreate(mintStr).t22 += toUiAmount(amount, 9);
      }
    }
  } catch {
    // No Token 2022 accounts
  }

  // 3. Hot balance via getAtaInterface (parallelize per mint)
  const mintKeys = [...mintMap.keys()];
  await Promise.allSettled(
    mintKeys.map(async (mintStr) => {
      try {
        const mint = new PublicKey(mintStr);
        const ata = getAssociatedTokenAddressInterface(mint, owner);
        const {parsed} = await getAtaInterface(rpc, ata, owner, mint);
        getOrCreate(mintStr).hot = toUiAmount(parsed.amount, 9);
      } catch {
        // Associated token account does not exist for this mint
      }
    }),
  );

  // 4. Cold balance (compressed token accounts — all mints at once)
  try {
    const compressed = await rpc.getCompressedTokenBalancesByOwnerV2(owner);
    for (const item of compressed.value.items) {
      const mintStr = item.mint.toBase58();
      getOrCreate(mintStr).cold += toUiAmount(BigInt(item.balance.toString()), 9);
    }
  } catch {
    // No compressed accounts
  }

  // Assemble result
  const tokens: TokenBalance[] = [];
  for (const [mintStr, entry] of mintMap) {
    tokens.push({
      mint: mintStr,
      decimals: entry.decimals,
      hot: entry.hot,
      cold: entry.cold,
      spl: entry.spl,
      t22: entry.t22,
      unified: entry.hot + entry.cold,
    });
  }

  return {sol: solLamports / LAMPORTS_PER_SOL, tokens};
}

function toBuffer(data: Buffer | Uint8Array | string | unknown): Buffer | null {
  if (data instanceof Buffer) return data;
  if (data instanceof Uint8Array) return Buffer.from(data);
  return null;
}

function toUiAmount(raw: bigint | {toNumber: () => number}, decimals: number): number {
  const value = typeof raw === 'bigint' ? Number(raw) : raw.toNumber();
  return value / 10 ** decimals;
}

export default getBalances;

// --- main ---
import {TREASURY_WALLET_ADDRESS} from './config.js';
getBalances(TREASURY_WALLET_ADDRESS)
  .then((b) => {
    console.log(`SOL: ${b.sol}`);
    if (b.tokens.length === 0) {
      console.log('No token balances found.');
      return;
    }
    for (const t of b.tokens) {
      const short = t.mint.slice(0, 6) + '...';
      console.log(`\nToken ${short} (unified): ${t.unified}`);
      console.log(`  Hot (Light Token associated token account): ${t.hot}`);
      console.log(`  Cold (compressed):     ${t.cold}`);
      console.log(`  SPL:                   ${t.spl}`);
      console.log(`  T22:                   ${t.t22}`);
    }
  })
  .catch(console.error);
