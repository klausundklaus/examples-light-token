# Using the Light-Token Standard vs SPL for Payments

**TL;DR**: Same API patterns, 1/200th ATA creation cost. Your users get the same USDC, just stored more efficiently.

---

## Quick Reference

| Operation      | SPL                                   | Light                                  |
| -------------- | ------------------------------------- | -------------------------------------- |
| Get/Create ATA | `getOrCreateAssociatedTokenAccount()` | `getOrCreateAtaInterface()`            |
| Derive ATA     | `getAssociatedTokenAddress()`         | `getAssociatedTokenAddressInterface()` |
| Transfer       | `transferChecked()`                   | `transferInterface()`                  |
| Get Balance    | `getAccount()`                        | `getAtaInterface()`                    |
| Tx History     | `getSignaturesForAddress()`           | `rpc.getSignaturesForOwnerInterface()` |
| Exit to SPL    | N/A                                   | `unwrap()`                             |

## Setup

```typescript
import { createRpc } from "@lightprotocol/stateless.js";

import {
    getOrCreateAtaInterface,
    getAtaInterface,
    getAssociatedTokenAddressInterface,
    transferInterface,
    unwrap,
} from "@lightprotocol/compressed-token/unified";

const rpc = createRpc(RPC_ENDPOINT);
```

---

## 1. Receive Payments

### Action

**SPL:**

```typescript
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

const ata = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    recipient
);
// Share ata.address with sender

console.log(ata.amount);
```

### Instruction

**SPL:**

```typescript
import {
    getAssociatedTokenAddressSync,
    createAssociatedTokenAccountIdempotentInstruction,
} from "@solana/spl-token";

const ata = getAssociatedTokenAddressSync(mint, recipient);

const tx = new Transaction().add(
    createAssociatedTokenAccountIdempotentInstruction(
        payer.publicKey,
        ata,
        recipient,
        mint
    )
);
```

**Light:**

```typescript
const ata = await getOrCreateAtaInterface(rpc, payer, mint, recipient);
// Share ata.parsed.address with sender

console.log(ata.parsed.amount);
```

**Light:**

```typescript
import {
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
    createLoadAtaInstructions,
    getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token/unified";
import { LIGHT_TOKEN_PROGRAM_ID } from "@lightprotocol/stateless.js";

const ata = getAssociatedTokenAddressInterface(mint, recipient);

const tx = new Transaction().add(
    createAssociatedTokenAccountInterfaceIdempotentInstruction(
        payer.publicKey,
        ata,
        recipient,
        mint,
        LIGHT_TOKEN_PROGRAM_ID
    ),
    ...(await createLoadAtaInstructions(
        rpc,
        ata,
        recipient,
        mint,
        payer.publicKey
    ))
);
```

---

## 2. Send Payments

### Action

**SPL:**

```typescript
import { transfer } from "@solana/spl-token";
const sourceAta = getAssociatedTokenAddressSync(mint, owner.publicKey);
const destinationAta = getAssociatedTokenAddressSync(mint, recipient);

await transfer(
    connection,
    payer,
    sourceAta,
    destinationAta,
    owner,
    amount,
    decimals
);
```

### Instruction

**SPL:**

```typescript
import {
    getAssociatedTokenAddressSync,
    createTransferInstruction,
} from "@solana/spl-token";

const sourceAta = getAssociatedTokenAddressSync(mint, owner.publicKey);
const destinationAta = getAssociatedTokenAddressSync(mint, recipient);

const tx = new Transaction().add(
    createTransferInstruction(
        sourceAta,
        destinationAta,
        owner.publicKey,
        amount
    )
);
```

**Light:**

```typescript
const sourceAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);

await transferInterface(rpc, payer, sourceAta, mint, recipient, owner, amount);
```

**Light (instruction-level):**

```typescript
import { Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import {
    createTransferInterfaceInstructions,
    sliceLast,
} from "@lightprotocol/compressed-token/unified";

const batches = await createTransferInterfaceInstructions(
    rpc,
    payer.publicKey,
    mint,
    amount,
    owner.publicKey,
    recipient
);
const { rest: loadBatches, last: transferBatch } = sliceLast(batches);

await Promise.all(
    loadBatches.map((batch) =>
        sendAndConfirmTransaction(rpc, new Transaction().add(...batch), [
            payer,
            owner,
        ])
    )
);
await sendAndConfirmTransaction(rpc, new Transaction().add(...transferBatch), [
    payer,
    owner,
]);
```

To ensure your recipient's ATA exists you can prepend an idempotent creation instruction in the same atomic transaction:

**SPL:**

```typescript
import {
    getAssociatedTokenAddressSync,
    createAssociatedTokenAccountIdempotentInstruction,
} from "@solana/spl-token";

const destinationAta = getAssociatedTokenAddressSync(mint, recipient);
const createAtaIx = createAssociatedTokenAccountIdempotentInstruction(
    payer.publicKey,
    destinationAta,
    recipient,
    mint
);

new Transaction().add(createAtaIx, transferIx);
```

**Light:**

```typescript
import {
    getAssociatedTokenAddressInterface,
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
} from "@lightprotocol/compressed-token/unified";
import { LIGHT_TOKEN_PROGRAM_ID } from "@lightprotocol/stateless.js";

const destinationAta = getAssociatedTokenAddressInterface(mint, recipient);
const createAtaIx = createAssociatedTokenAccountInterfaceIdempotentInstruction(
    payer.publicKey,
    destinationAta,
    recipient,
    mint,
    LIGHT_TOKEN_PROGRAM_ID
);

new Transaction().add(createAtaIx, transferIx);
```

---

## 3. Show Balance

### Action

**SPL:**

```typescript
import { getAccount } from "@solana/spl-token";

const account = await getAccount(connection, ata);
console.log(account.amount);
```

**Light:**

```typescript
const ata = getAssociatedTokenAddressInterface(mint, owner);
const account = await getAtaInterface(rpc, ata, owner, mint);

console.log(account.parsed.amount);
```

---

## 4. Transaction History

### Action

**SPL:**

```typescript
const signatures = await connection.getSignaturesForAddress(ata);
```

**Light:**

```typescript
// Unified: fetches both on-chain and compressed tx signatures
const result = await rpc.getSignaturesForOwnerInterface(owner);

console.log(result.signatures); // Merged + deduplicated
console.log(result.solana); // On-chain txs only
console.log(result.compressed); // Compressed txs only
```

Use `getSignaturesForAddressInterface(address)` if you want address-specific rather than owner-wide history.

---

## 5. Unwrap to SPL

When users need vanilla SPL tokens (eg., for CEX off-ramp):

### Action

**Light -> SPL ATA:**

```typescript
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

// SPL ATA must exist
const splAta = getAssociatedTokenAddressSync(mint, owner.publicKey);

await unwrap(rpc, payer, splAta, owner, mint, amount);
```

### Instruction

**Light:**

```typescript
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { createUnwrapInstructions } from "@lightprotocol/compressed-token/unified";

const splAta = getAssociatedTokenAddressSync(mint, owner.publicKey);

// Handles loading cold state + unwrapping in one go.
const instructions = await createUnwrapInstructions(
    rpc,
    splAta,
    owner.publicKey,
    mint,
    amount,
    payer.publicKey
);

for (const ixs of instructions) {
    const tx = new Transaction().add(...ixs);
    await sendAndConfirmTransaction(rpc, tx, [payer, owner]);
}
```
