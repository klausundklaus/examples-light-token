# Squads Smart Wallet + Light Token Integration

This toolkit demonstrates how to use rent-free Light Tokens with Squads Protocol v4 multisig vaults. Squads vaults are standard Solana PDAs, which makes them fully compatible with the Light Token interface system.

## Overview

Light Tokens use real Solana ATAs (Associated Token Accounts) with protocol-sponsored rent. Squads vaults are PDAs that can own these ATAs. This means you can:

- Hold rent-free tokens in a multisig-controlled vault
- Transfer Light Tokens to and from vaults using the standard interface
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

## Transfers TO Vault

Transferring Light Tokens into a vault works exactly like any other transfer. The vault PDA is just a regular `PublicKey` recipient:

```typescript
import { transferInterface } from "@lightprotocol/compressed-token/unified";

await transferInterface(rpc, payer, sourceAta, mint, vaultPda, payer, amount);
```

This creates a Light Token ATA owned by the vault PDA if one does not exist, or transfers into the existing one.

See `transfer-to-vault.ts` for a complete example.

## Transfers FROM Vault

Transfers from a vault require wrapping the Light Token transfer instructions in a Squads vault transaction. The vault PDA "signs" via CPI inside the Squads program, so you never need a keypair for it.

### Step 1: Build Light Token transfer instructions

```typescript
import { createTransferInterfaceInstructions } from "@lightprotocol/compressed-token/unified";

const ixBatches = await createTransferInterfaceInstructions(
    rpc,
    vaultPda,   // payer for the inner tx
    mint,
    amount,
    vaultPda,   // owner of the source ATA
    recipient
);
```

The function accepts `owner: PublicKey` (not `Signer`), which allows PDA owners.

### Step 2: Wrap in a Squads vault transaction

```typescript
import { TransactionMessage } from "@solana/web3.js";

for (const ixs of ixBatches) {
    const message = new TransactionMessage({
        payerKey: vaultPda,
        recentBlockhash: (await rpc.getLatestBlockhash()).blockhash,
        instructions: ixs,
    });

    await multisig.rpc.vaultTransactionCreate({
        connection: rpc,
        feePayer: payer,
        multisigPda,
        transactionIndex,
        creator: payer.publicKey,
        vaultIndex: 0,
        ephemeralSigners: 0,
        transactionMessage: message,
    });
}
```

### Step 3: Propose, approve, execute

```typescript
await multisig.rpc.proposalCreate({ connection: rpc, feePayer: payer, multisigPda, transactionIndex, creator: payer });
await multisig.rpc.proposalApprove({ connection: rpc, feePayer: payer, multisigPda, transactionIndex, member: payer });
await multisig.rpc.vaultTransactionExecute({ connection: rpc, feePayer: payer, multisigPda, transactionIndex, member: payer.publicKey, signers: [payer] });
```

See `transfer-from-vault.ts` for a complete example.

## SPL Transfers FROM Vault

You can unwrap Light Tokens back to standard SPL inside the vault, then transfer the SPL tokens out via a Squads vault transaction. This is useful when interacting with protocols that only accept standard SPL tokens.

1. Unwrap Light Tokens to the vault's SPL ATA
2. Build a standard SPL `createTransferInstruction`
3. Wrap in a Squads vault transaction and execute

See `transfer-spl-from-vault.ts` for a complete example.

## Running the Examples

All examples are self-contained and use localnet by default. To run against devnet, uncomment the devnet RPC configuration at the top of each file.

```bash
npm install

# Transfer light tokens to a vault
npx tsx transfer-to-vault.ts

# Transfer light tokens from a vault
npx tsx transfer-from-vault.ts

# Unwrap and transfer SPL from a vault
npx tsx transfer-spl-from-vault.ts

# Run integration test
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
- `@solana/spl-token` - SPL Token program (peer dependency)
