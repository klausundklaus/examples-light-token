# Squads Smart Wallet + Light Token Integration

This toolkit demonstrates how to use rent-free Light Tokens with Squads Protocol v4 multisig vaults. Squads vaults are standard Solana PDAs, which makes them fully compatible with the Light Token interface system.

## Overview

Light Tokens use real Solana ATAs (Associated Token Accounts) with protocol-sponsored rent. Squads vaults are PDAs that can own these ATAs. This means you can:

- Hold rent-free tokens in a multisig-controlled vault
- Transfer Light Tokens to and from vaults
- Unwrap Light Tokens back to SPL/T22 inside a vault and transfer as normal SPL

## Key Concept: Vault PDA as Token Owner

A Squads vault PDA is derived from the multisig PDA:

```typescript
import * as multisig from "@sqds/multisig";

const [multisigPda] = multisig.getMultisigPda({ createKey: createKey.publicKey });
const [vaultPda] = multisig.getVaultPda({ multisigPda, index: 0 });
```

Since the vault PDA is off-curve (not a valid keypair), you must set `allowOwnerOffCurve = true` when creating or deriving ATAs for it:

```typescript
import { createAtaInterface, getAssociatedTokenAddressInterface } from "@lightprotocol/compressed-token";

await createAtaInterface(rpc, payer, mint, vaultPda, true);
const vaultAta = getAssociatedTokenAddressInterface(mint, vaultPda, true);
```

## Important: Off-Curve PDA Transfers

The high-level SDK functions `transferInterface()` and `createTransferInterfaceInstructions()` enforce on-curve validation for recipients and owners, which means they **reject Squads vault PDAs** (and any other off-curve PDA).

Use `createLightTokenTransferInstruction()` instead — it builds a raw ATA-to-ATA transfer instruction that accepts any `PublicKey` for source, destination, and owner:

```typescript
import {
    createLightTokenTransferInstruction,
    getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";
import { buildAndSignTx, sendAndConfirmTx } from "@lightprotocol/stateless.js";

const ix = createLightTokenTransferInstruction(
    sourceAta,     // source Light Token ATA
    destAta,       // destination Light Token ATA
    ownerPubkey,   // owner of the source ATA (can be off-curve PDA)
    amount,
    feePayer       // optional, defaults to owner
);

const { blockhash } = await rpc.getLatestBlockhash();
const tx = buildAndSignTx([ix], payer, blockhash, []);
await sendAndConfirmTx(rpc, tx);
```

## Transfers TO Vault

Transfer Light Tokens into a vault using `createLightTokenTransferInstruction`:

```typescript
const vaultAta = getAssociatedTokenAddressInterface(mint, vaultPda, true);

const ix = createLightTokenTransferInstruction(
    payerAta,          // source
    vaultAta,          // destination (off-curve vault PDA)
    payer.publicKey,   // owner of source
    amount
);

const { blockhash } = await rpc.getLatestBlockhash();
const tx = buildAndSignTx([ix], payer, blockhash, []);
await sendAndConfirmTx(rpc, tx);
```

See `transfer-to-vault.ts` for a complete example.

## Transfers FROM Vault

Transfers from a vault require wrapping the instruction in a Squads vault transaction. The vault PDA "signs" via CPI inside the Squads program.

### Step 1: Build the transfer instruction

```typescript
const ix = createLightTokenTransferInstruction(
    vaultAta,          // source (vault's Light Token ATA)
    recipientAta,      // destination
    vaultPda,          // owner (off-curve PDA)
    amount,
    vaultPda           // fee payer for the inner tx
);
```

### Step 2: Wrap in a Squads vault transaction

```typescript
import { TransactionMessage } from "@solana/web3.js";

const message = new TransactionMessage({
    payerKey: vaultPda,
    recentBlockhash: (await rpc.getLatestBlockhash()).blockhash,
    instructions: [ix],
});

const vtSig = await multisig.rpc.vaultTransactionCreate({
    connection: rpc,
    feePayer: payer,
    multisigPda,
    transactionIndex: txIndex,
    creator: payer.publicKey,
    vaultIndex: 0,
    ephemeralSigners: 0,
    transactionMessage: message,
});
await rpc.confirmTransaction(vtSig, "confirmed");
```

### Step 3: Propose, approve, execute

Each step must be confirmed before the next (especially important on devnet):

```typescript
const proposalSig = await multisig.rpc.proposalCreate({
    connection: rpc, feePayer: payer, multisigPda, transactionIndex: txIndex, creator: payer,
});
await rpc.confirmTransaction(proposalSig, "confirmed");

const approveSig = await multisig.rpc.proposalApprove({
    connection: rpc, feePayer: payer, multisigPda, transactionIndex: txIndex, member: payer,
});
await rpc.confirmTransaction(approveSig, "confirmed");

await multisig.rpc.vaultTransactionExecute({
    connection: rpc, feePayer: payer, multisigPda, transactionIndex: txIndex,
    member: payer.publicKey, signers: [payer],
});
```

**Note**: The vault must hold enough SOL to pay for the inner transaction fees. Fund it before creating the vault transaction:

```typescript
import { SystemProgram, Transaction } from "@solana/web3.js";

const solTx = new Transaction().add(
    SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: vaultPda,
        lamports: 10_000_000, // 0.01 SOL
    })
);
```

See `transfer-from-vault.ts` for a complete example.

## SPL Transfers FROM Vault

You can unwrap Light Tokens back to standard SPL inside the vault, then transfer the SPL tokens out via a Squads vault transaction. This is useful when interacting with protocols that only accept standard SPL tokens.

1. Fund vault with Light Tokens using `createLightTokenTransferInstruction`
2. Unwrap Light Tokens to the vault's SPL ATA via `createUnwrapInstructions`
3. Build a standard SPL `createTransferInstruction`
4. Wrap each step in a Squads vault transaction and execute

See `transfer-spl-from-vault.ts` for a complete example.

## Running the Examples

All examples use `RPC_URL` environment variable (defaults to `http://127.0.0.1:8899` for localnet).

```bash
npm install

# Run against devnet
export RPC_URL="https://devnet.helius-rpc.com?api-key=YOUR_KEY"

# Transfer Light Tokens to a vault
npx tsx transfer-to-vault.ts

# Transfer Light Tokens from a vault
npx tsx transfer-from-vault.ts

# Unwrap and transfer SPL from a vault
npx tsx transfer-spl-from-vault.ts

# Run integration test (full end-to-end flow)
npx tsx squads-light-token.test.ts
```

## Creating the Multisig

All examples create a 1-of-1 multisig for simplicity. For production use, configure multiple members and a higher threshold:

```typescript
const { Permissions } = multisig.types;

await multisig.rpc.multisigCreateV2({
    connection: rpc,
    createKey,
    creator: payer,
    multisigPda,
    configAuthority: null,
    timeLock: 0,
    members: [
        { key: member1.publicKey, permissions: Permissions.all() },
        { key: member2.publicKey, permissions: Permissions.all() },
        { key: member3.publicKey, permissions: Permissions.all() },
    ],
    threshold: 2,
    rentCollector: null,
    treasury: programConfig.treasury,
});
```

## Dependencies

- `@lightprotocol/compressed-token` - Light Token SDK
- `@lightprotocol/stateless.js` - Light Protocol RPC client
- `@sqds/multisig` - Squads Protocol v4 SDK
- `@solana/web3.js` - Solana web3 (peer dependency)
- `@solana/spl-token` - SPL Token program (for unwrap/SPL transfer examples)
