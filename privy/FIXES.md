# Proposed fixes for `privy/nodejs/src/`

Analysis of the `add-privy` branch against the Light Protocol SDK internals and load-ata test suite. Each issue includes severity, affected files, root cause, and proposed fix with exact code changes.

## Issue 1: No handling of >8 cold accounts

**Severity:** High
**Files:** `load.ts`, `transfer.ts`

### Root cause

`createLoadAtaInstructionsFromInterface()` returns instructions for up to 8 compressed accounts per call. The Light Protocol SDK reference implementation (`load-ata.ts:539`) handles this by calling `loadAta` in a loop until it returns `null`. Both `load.ts` and `transfer.ts` call the function once and assume all accounts are consolidated.

If an owner has 9+ cold compressed accounts, the first call consolidates 8 and the remaining stay cold. In `transfer.ts`, this means the transfer may fail with insufficient balance.

### Proposed fix — load.ts

Replace single-call pattern with a loop. Each iteration builds, signs, and sends a transaction for up to 8 accounts. Loop continues until no instructions are returned.

```typescript
// BEFORE (load.ts lines 34-67)
const ixs = await createLoadAtaInstructionsFromInterface(
  connection,
  ownerPubkey,
  ataInfo,
  undefined,
  true,
);

if (ixs.length === 0) {
  console.log('Nothing to load');
  return null;
}

const transaction = new Transaction();
transaction.add(...ixs);
// ... sign and send single transaction
```

```typescript
// AFTER
const signatures: string[] = [];

while (true) {
  // Re-query each iteration — previous tx changed on-chain state
  const ataInfo = await getAtaInterface(connection, lightTokenAta, ownerPubkey, mintPubkey);

  const ixs = await createLoadAtaInstructionsFromInterface(
    connection,
    ownerPubkey,
    ataInfo,
    undefined,
    true,
  );

  if (ixs.length === 0) break;

  const transaction = new Transaction();
  transaction.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }));
  transaction.add(...ixs);

  const { blockhash } = await connection.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.feePayer = ownerPubkey;

  // Sign with Privy
  const signResult = await privy.wallets().solana().signTransaction(
    process.env.TREASURY_WALLET_ID!,
    {
      transaction: transaction.serialize({ requireAllSignatures: false }),
      authorization_context: {
        authorization_private_keys: [process.env.TREASURY_AUTHORIZATION_KEY!],
      },
    },
  );
  const signedTx = (signResult as any).signed_transaction;
  if (!signedTx) {
    throw new Error('Privy returned invalid response: ' + JSON.stringify(signResult));
  }

  const signature = await connection.sendRawTransaction(
    Buffer.from(signedTx, 'base64'),
    { skipPreflight: false, preflightCommitment: 'confirmed' },
  );
  await connection.confirmTransaction(signature, 'confirmed');
  signatures.push(signature);
}

if (signatures.length === 0) {
  console.log('Nothing to load');
  return null;
}

return signatures;
```

**Key changes:**
- Move `getAtaInterface()` call inside the loop (on-chain state changes after each tx)
- Add `ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 })` (matches SDK reference)
- Return `string[]` instead of `string | null`
- Loop exits when `ixs.length === 0`

**Import additions:**
```typescript
import { ComputeBudgetProgram } from '@solana/web3.js';
```

### Proposed fix — transfer.ts

Transfer is more constrained — the load + transfer must happen atomically in one transaction. The approach: loop to consolidate all cold accounts first (separate transactions), then do load-remaining + transfer in the final transaction.

```typescript
// BEFORE (transfer.ts lines 55-65)
try {
  const ataInfo = await getAtaInterface(connection, sourceAta, fromPubkey, mintPubkey);
  const loadIxs = await createLoadAtaInstructionsFromInterface(
    connection, fromPubkey, ataInfo, undefined, true,
  );
  if (loadIxs.length > 0) transaction.add(...loadIxs);
} catch {
  // Associated token account doesn't exist yet
}
```

```typescript
// AFTER

// Phase 1: Consolidate all cold accounts (may require multiple transactions)
let hasMoreToLoad = true;
while (hasMoreToLoad) {
  let ataInfo;
  try {
    ataInfo = await getAtaInterface(connection, sourceAta, fromPubkey, mintPubkey);
  } catch (e) {
    if (e instanceof TokenAccountNotFoundError) break;
    throw e;
  }

  const loadIxs = await createLoadAtaInstructionsFromInterface(
    connection, fromPubkey, ataInfo, undefined, true,
  );

  if (loadIxs.length === 0) {
    hasMoreToLoad = false;
    break;
  }

  // Check if there are likely more accounts after this batch
  // If we got a full batch (8 accounts), there may be more
  const coldCount = ataInfo.parsed?.compressedAccounts?.length ?? 0;
  if (coldCount <= 8) {
    // Last batch — include these in the final transaction with the transfer
    transaction.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }));
    transaction.add(...loadIxs);
    hasMoreToLoad = false;
  } else {
    // More batches needed — send this one separately
    const loadTx = new Transaction();
    loadTx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }));
    loadTx.add(...loadIxs);

    const { blockhash } = await connection.getLatestBlockhash();
    loadTx.recentBlockhash = blockhash;
    loadTx.feePayer = fromPubkey;

    // Sign and send load transaction
    const signResult = await privy.wallets().solana().signTransaction(
      process.env.TREASURY_WALLET_ID!,
      {
        transaction: loadTx.serialize({ requireAllSignatures: false }),
        authorization_context: {
          authorization_private_keys: [process.env.TREASURY_AUTHORIZATION_KEY!],
        },
      },
    );
    const signedTx = (signResult as any).signed_transaction;
    if (!signedTx) throw new Error('Privy returned invalid response');
    const sig = await connection.sendRawTransaction(Buffer.from(signedTx, 'base64'), {
      skipPreflight: false, preflightCommitment: 'confirmed',
    });
    await connection.confirmTransaction(sig, 'confirmed');
  }
}
```

**Note:** This is the most complex fix. An alternative simpler approach: always consolidate all cold accounts first (separate transactions), then do a transfer-only transaction. This avoids combining load + transfer in one tx but requires an extra transaction when cold accounts exist. Simpler to implement and reason about.

**Simpler alternative:**

```typescript
// Phase 1: Consolidate all cold accounts first
try {
  while (true) {
    const ataInfo = await getAtaInterface(connection, sourceAta, fromPubkey, mintPubkey);
    const loadIxs = await createLoadAtaInstructionsFromInterface(
      connection, fromPubkey, ataInfo, undefined, true,
    );
    if (loadIxs.length === 0) break;

    const loadTx = new Transaction();
    loadTx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }));
    loadTx.add(...loadIxs);

    const { blockhash } = await connection.getLatestBlockhash();
    loadTx.recentBlockhash = blockhash;
    loadTx.feePayer = fromPubkey;

    // Sign and send (Privy signing code)
    // ... same pattern as existing
  }
} catch (e) {
  if (!(e instanceof TokenAccountNotFoundError)) throw e;
}

// Phase 2: Transfer (all funds now in light-token ATA)
transaction.add(createTransferInterfaceInstruction(sourceAta, destAta, fromPubkey, tokenAmount));
```

**Recommendation:** Use the simpler alternative. It separates concerns cleanly: consolidate first, then transfer.

---

## Issue 2: Silent error swallowing in transfer.ts

**Severity:** Medium
**File:** `transfer.ts`

### Root cause

Bare `catch {}` at line 63 catches all errors, including network timeouts, RPC failures, and invalid addresses. Only `TokenAccountNotFoundError` (account doesn't exist yet) is expected.

### Proposed fix

```typescript
// BEFORE (transfer.ts lines 55-65)
try {
  const ataInfo = await getAtaInterface(connection, sourceAta, fromPubkey, mintPubkey);
  const loadIxs = await createLoadAtaInstructionsFromInterface(
    connection, fromPubkey, ataInfo, undefined, true,
  );
  if (loadIxs.length > 0) transaction.add(...loadIxs);
} catch {
  // Associated token account doesn't exist yet
}
```

```typescript
// AFTER
import { TokenAccountNotFoundError } from '@solana/spl-token';

try {
  const ataInfo = await getAtaInterface(connection, sourceAta, fromPubkey, mintPubkey);
  const loadIxs = await createLoadAtaInstructionsFromInterface(
    connection, fromPubkey, ataInfo, undefined, true,
  );
  if (loadIxs.length > 0) {
    transaction.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }));
    transaction.add(...loadIxs);
  }
} catch (e) {
  if (!(e instanceof TokenAccountNotFoundError)) throw e;
  // Light-token ATA doesn't exist — transfer from hot balance only
}
```

**Import addition:**
```typescript
import { TokenAccountNotFoundError } from '@solana/spl-token';
```

**Reference:** `unwrap.ts` already uses this pattern with `TokenAccountNotFoundError` for ATA existence checks.

**Note:** If Issue 1 (loop) is also applied, this catch block becomes part of the loop structure shown above. The fixes compose — apply Issue 2's error specificity within Issue 1's loop.

---

## Issue 3: Hardcoded decimals in balances.ts

**Severity:** Medium
**File:** `balances.ts`

### Root cause

Line 51: `const decimals = 9;` — hardcoded. Returns wrong UI amounts for any mint with non-9 decimals (e.g., USDC has 6).

### Proposed fix

Query mint decimals from the token program. The `getMint()` function from `@solana/spl-token` returns a `Mint` object with a `.decimals` field. Since this mint could be SPL or T22, query both programs and use whichever succeeds.

```typescript
// BEFORE (balances.ts line 51)
const decimals = 9;
```

```typescript
// AFTER
import { getMint } from '@solana/spl-token';

let decimals = 9; // fallback
try {
  const mintInfo = await getMint(rpc, mint, undefined, TOKEN_PROGRAM_ID);
  decimals = mintInfo.decimals;
} catch {
  try {
    const mintInfo = await getMint(rpc, mint, undefined, TOKEN_2022_PROGRAM_ID);
    decimals = mintInfo.decimals;
  } catch {
    // Light-only mint — no SPL/T22 program. Default to 9.
    // Light mints don't have on-chain decimals metadata via getMint.
  }
}
```

**Import addition:**
```typescript
import { TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, getMint } from '@solana/spl-token';
```

(`TOKEN_PROGRAM_ID` and `TOKEN_2022_PROGRAM_ID` are already imported.)

**Alternative:** Accept `decimals` as a function parameter with default 9:

```typescript
export async function getBalances(
  ownerAddress: string,
  mintAddress?: string,
  decimals: number = 9,  // caller can override
): Promise<BalanceBreakdown> {
```

This is simpler and avoids the extra RPC call. The caller knows the mint's decimals. Both approaches are valid — the `getMint` approach is self-contained, the parameter approach is simpler.

**Recommendation:** Use the `getMint` approach. The function is already making multiple RPC calls; one more is negligible, and it eliminates a class of caller errors.

---

## Issue 4: Missing compute budget in transfer.ts

**Severity:** Medium
**File:** `transfer.ts`

### Root cause

`wrap.ts` and `unwrap.ts` both set `ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 })`. `transfer.ts` omits this despite including load instructions that decompress compressed accounts — an operation the SDK reference allocates 500,000 CU for.

Without explicit CU allocation, the transaction uses the default 200,000 CU. A load + transfer combination can exceed this, causing the transaction to fail with `ComputeBudgetExceeded`.

### Proposed fix

This fix is included in the Issue 1 and Issue 2 code above. Add compute budget when load instructions are present:

```typescript
// BEFORE — no compute budget set
transaction.add(...loadIxs);

// AFTER — set 500k CU when loading
if (loadIxs.length > 0) {
  transaction.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }));
  transaction.add(...loadIxs);
}
```

**Import addition:**
```typescript
import { PublicKey, Transaction, ComputeBudgetProgram } from '@solana/web3.js';
```

(`ComputeBudgetProgram` added to existing import.)

**Reference:** Light Protocol SDK uses 500,000 CU in `load-ata.ts:539`. The privy branch's `wrap.ts` and `unwrap.ts` use 200,000 — this is sufficient for wrap/unwrap but not for decompression + wrap + transfer.

---

## Issue 5: Amount type inconsistency in transfer.ts

**Severity:** Low-medium
**File:** `transfer.ts`

### Root cause

Line 35: `const tokenAmount = Math.floor(amount * Math.pow(10, decimals));`

This produces a `number`. JavaScript `number` is IEEE 754 double-precision, which loses integer precision above 2^53 (9,007,199,254,740,992). For a 9-decimal token, this means amounts above ~9,007,199.254 tokens lose precision.

`wrap.ts` and `unwrap.ts` correctly use `BigInt`:
```typescript
const tokenAmount = BigInt(Math.floor(amount * Math.pow(10, decimals)));
```

### Proposed fix

```typescript
// BEFORE (transfer.ts line 35)
const tokenAmount = Math.floor(amount * Math.pow(10, decimals));

// AFTER
const tokenAmount = BigInt(Math.floor(amount * Math.pow(10, decimals)));
```

Also update the `createTransferInterfaceInstruction` call if it expects `bigint`:

```typescript
// BEFORE (transfer.ts line 68)
transaction.add(createTransferInterfaceInstruction(sourceAta, destAta, fromPubkey, tokenAmount));

// AFTER — no change needed if the function accepts both number and bigint
// Verify the function signature accepts bigint
```

**Note:** Check whether `createTransferInterfaceInstruction` accepts `number | bigint` or only `number`. If it only accepts `number`, the `BigInt` conversion is unnecessary at this call site, but the inconsistency with wrap/unwrap should still be resolved for the public API (`amount` parameter type).

---

## Issue 6: Privy response type assertion

**Severity:** Low
**Files:** `load.ts`, `transfer.ts`, `wrap.ts`, `unwrap.ts`

### Root cause

All four files use:
```typescript
const signedTx = (signResult as any).signed_transaction;
```

The `as any` cast bypasses TypeScript's type system. If `@privy-io/node` changes the response shape (likely — it's `0.1.0-alpha.2`), this silently returns `undefined`, and the `if (!signedTx)` check throws a generic error.

### Proposed fix

Extract a shared helper in `config.ts`:

```typescript
// config.ts — add at bottom

export async function signWithPrivy(
  privy: PrivyClient,
  walletId: string,
  transaction: Transaction,
  authorizationKey: string,
): Promise<Buffer> {
  const signResult = await privy.wallets().solana().signTransaction(walletId, {
    transaction: transaction.serialize({ requireAllSignatures: false }),
    authorization_context: {
      authorization_private_keys: [authorizationKey],
    },
  });

  // Type assertion — @privy-io/node@0.1.0-alpha.2 doesn't export response types
  // TODO: Replace with typed response when Privy SDK stabilizes
  const signedTx = (signResult as Record<string, unknown>).signed_transaction;
  if (typeof signedTx !== 'string') {
    throw new Error(
      `Privy signTransaction returned unexpected shape. ` +
      `Expected { signed_transaction: string }, got: ${JSON.stringify(signResult)}`,
    );
  }

  return Buffer.from(signedTx, 'base64');
}
```

Then in each file, replace the duplicated signing block:

```typescript
// BEFORE (repeated in load.ts, transfer.ts, wrap.ts, unwrap.ts)
const signResult = await privy.wallets().solana().signTransaction(process.env.TREASURY_WALLET_ID!, {
  transaction: transaction.serialize({requireAllSignatures: false}),
  authorization_context: {
    authorization_private_keys: [process.env.TREASURY_AUTHORIZATION_KEY!]
  }
});
const signedTx = (signResult as any).signed_transaction;
if (!signedTx) {
  throw new Error('Privy returned invalid response: ' + JSON.stringify(signResult));
}
const signedTransaction = Buffer.from(signedTx, 'base64');

// AFTER
import { signWithPrivy, TREASURY_WALLET_ID, TREASURY_AUTHORIZATION_KEY } from './config.js';

const signedTransaction = await signWithPrivy(
  privy, TREASURY_WALLET_ID, transaction, TREASURY_AUTHORIZATION_KEY,
);
```

**Benefits:**
- Single place to update when Privy SDK changes
- `Record<string, unknown>` instead of `any` — slightly safer
- `typeof` check instead of truthiness check — catches `0`, `false`, etc.
- Better error message with expected vs actual shape
- Removes ~10 lines of duplicated code per file

**Recommendation:** Apply this even though the SDK is alpha. The deduplication alone justifies it — 4 copies of the same signing code is a maintenance burden.

---

## Summary

| # | Issue | Severity | File(s) | Type |
|---|-------|----------|---------|------|
| 1 | No >8 cold account loop | High | `load.ts`, `transfer.ts` | Architectural |
| 2 | Silent catch-all | Medium | `transfer.ts` | Error handling |
| 3 | Hardcoded decimals | Medium | `balances.ts` | Correctness |
| 4 | Missing compute budget | Medium | `transfer.ts` | Reliability |
| 5 | Number vs BigInt | Low-medium | `transfer.ts` | Type safety |
| 6 | Privy `as any` cast | Low | All 4 action files + `config.ts` | Maintainability |

### Dependency order

Issues 1, 2, and 4 overlap in `transfer.ts`. Apply them together:

1. **Issue 6** first — extract `signWithPrivy` helper to `config.ts` (reduces diff noise for other fixes)
2. **Issues 1 + 2 + 4** together in `transfer.ts` (loop + specific errors + compute budget)
3. **Issue 1** in `load.ts` (loop + compute budget, uses `signWithPrivy` from step 1)
4. **Issue 3** in `balances.ts` (independent)
5. **Issue 5** in `transfer.ts` (one-line change, apply last)

### Files modified

| File | Issues |
|------|--------|
| `privy/nodejs/src/config.ts` | #6 (add `signWithPrivy` helper) |
| `privy/nodejs/src/load.ts` | #1, #6 |
| `privy/nodejs/src/transfer.ts` | #1, #2, #4, #5, #6 |
| `privy/nodejs/src/balances.ts` | #3 |
| `privy/nodejs/src/wrap.ts` | #6 |
| `privy/nodejs/src/unwrap.ts` | #6 |
