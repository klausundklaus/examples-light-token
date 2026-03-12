# Squads Smart Account + Light Token Integration

This toolkit demonstrates how to use rent-free Light Tokens with Squads Smart Accounts — programmable wallets with configurable access control on Solana.

## Overview

A Squads Smart Account is a wallet (PDA) with rules: who can sign, at what threshold, with what time lock. Light Tokens are real Solana ATAs with protocol-sponsored rent. Combined, you get a programmable wallet that holds rent-free tokens.

Two execution modes:

- **Sync** — Immediate execution in a single transaction. Requires all signers present and `timeLock=0`. No proposal overhead.
- **Async** — Full proposal lifecycle: create → propose → approve → execute. For multi-party governance.

## Smart Account Setup

```typescript
import * as smartAccount from "@sqds/smart-account";

// Read ProgramConfig to get the next available account index
const programConfig =
    await smartAccount.accounts.ProgramConfig.fromAccountAddress(
        rpc,
        smartAccount.getProgramConfigPda({})[0]
    );
const accountIndex =
    BigInt(programConfig.smartAccountIndex.toString()) + 1n;

// Derive PDAs
const [settingsPda] = smartAccount.getSettingsPda({ accountIndex });
const [walletPda] = smartAccount.getSmartAccountPda({
    settingsPda,
    accountIndex: 0,
});

// Create 1-of-1 smart account (timeLock=0 enables sync execution)
await smartAccount.rpc.createSmartAccount({
    connection: rpc,
    treasury: programConfig.treasury,
    creator: payer,
    settings: settingsPda,
    settingsAuthority: null,
    threshold: 1,
    signers: [
        { key: payer.publicKey, permissions: smartAccount.types.Permissions.all() },
    ],
    timeLock: 0,
    rentCollector: null,
});
```

For multi-party governance, add more signers and increase the threshold:

```typescript
const { Permission, Permissions } = smartAccount.types;

signers: [
    { key: admin.publicKey, permissions: Permissions.all() },
    { key: signer2.publicKey, permissions: Permissions.fromPermissions([Permission.Vote]) },
    { key: signer3.publicKey, permissions: Permissions.fromPermissions([Permission.Vote]) },
],
threshold: 2,
```

## Off-Curve PDA Transfers

The wallet PDA is off-curve (not a valid keypair). The high-level SDK functions `transferInterface()` and `createTransferInterfaceInstructions()` reject off-curve addresses.

Use `createLightTokenTransferInstruction()` instead — it accepts any `PublicKey`:

```typescript
import { createLightTokenTransferInstruction } from "@lightprotocol/compressed-token";

const ix = createLightTokenTransferInstruction(
    sourceAta,     // source Light Token ATA
    destAta,       // destination Light Token ATA
    ownerPubkey,   // owner of source ATA (can be off-curve PDA)
    amount,
    feePayer       // optional, defaults to owner
);
```

## Fund the Smart Wallet

Anyone can send Light Tokens to a smart wallet — no approval needed.

```typescript
// Create wallet's Light Token ATA (allowOwnerOffCurve=true for PDAs)
await createAtaInterface(rpc, payer, mint, walletPda, true);
const walletAta = getAssociatedTokenAddressInterface(mint, walletPda, true);

// Transfer
const ix = createLightTokenTransferInstruction(
    payerAta, walletAta, payer.publicKey, amount
);
const { blockhash } = await rpc.getLatestBlockhash();
const tx = buildAndSignTx([ix], payer, blockhash, []);
await sendAndConfirmTx(rpc, tx);
```

See `fund-wallet.ts` for a complete example.

## Smart Wallet Sends — Sync Execution

Single transaction, immediate execution. The smart account program executes the inner instruction via CPI, signing with the wallet PDA's seeds.

```typescript
// Build Light Token transfer instruction
const transferIx = createLightTokenTransferInstruction(
    walletAta, recipientAta, walletPda, amount, walletPda
);

// Compile for synchronous execution
const { instructions, accounts } =
    smartAccount.utils.instructionsToSynchronousTransactionDetails({
        vaultPda: walletPda,
        members: [payer.publicKey],
        transaction_instructions: [transferIx],
    });

// Build sync execution instruction
const syncIx = smartAccount.instructions.executeTransactionSync({
    settingsPda,
    numSigners: 1,
    accountIndex: 0,
    instructions,
    instruction_accounts: accounts,
});

// Send as a single transaction
const msg = new TransactionMessage({
    payerKey: payer.publicKey,
    recentBlockhash: blockhash,
    instructions: [syncIx],
}).compileToV0Message();
const tx = new VersionedTransaction(msg);
tx.sign([payer]);
await rpc.sendRawTransaction(tx.serialize());
```

See `wallet-send-sync.ts` for a complete example.

## Smart Wallet Sends — Async Proposal Flow

Multi-step governance flow. Each step must be confirmed before the next.

```typescript
// Read current transaction index
const settings = await smartAccount.accounts.Settings.fromAccountAddress(
    rpc, settingsPda
);
const txIndex = BigInt(settings.transactionIndex.toString()) + 1n;

// 1. Create transaction
await smartAccount.rpc.createTransaction({
    connection: rpc, feePayer: payer, settingsPda,
    transactionIndex: txIndex, creator: payer.publicKey,
    accountIndex: 0, ephemeralSigners: 0,
    transactionMessage: new TransactionMessage({
        payerKey: walletPda,
        recentBlockhash: blockhash,
        instructions: [transferIx],
    }),
});

// 2. Create proposal
await smartAccount.rpc.createProposal({
    connection: rpc, feePayer: payer, settingsPda,
    transactionIndex: txIndex, creator: payer,
});

// 3. Approve (repeat for each signer up to threshold)
await smartAccount.rpc.approveProposal({
    connection: rpc, feePayer: payer, settingsPda,
    transactionIndex: txIndex, signer: payer,
});

// 4. Execute
await smartAccount.rpc.executeTransaction({
    connection: rpc, feePayer: payer, settingsPda,
    transactionIndex: txIndex, signer: payer.publicKey,
    signers: [payer],
});
```

See `wallet-send-async.ts` for a complete example.

## Running the Examples

```bash
npm install

# Set RPC endpoint (defaults to localhost)
export RPC_URL="https://devnet.helius-rpc.com?api-key=YOUR_KEY"

# Fund a smart wallet with Light Tokens
npx tsx fund-wallet.ts

# Smart wallet sends LTs (sync — single transaction)
npx tsx wallet-send-sync.ts

# Smart wallet sends LTs (async — proposal flow)
npx tsx wallet-send-async.ts

# Run full integration test (all 3 flows)
npx tsx squads-light-token.test.ts
```

## Dependencies

- `@lightprotocol/compressed-token` — Light Token SDK
- `@lightprotocol/stateless.js` — Light Protocol RPC client
- `@sqds/smart-account` — Squads Smart Account SDK
- `@solana/web3.js` — Solana web3 (peer dependency)

## Program IDs

| Program | ID |
|---------|-----|
| Squads Smart Account | `SMRTzfY6DfH5ik3TKiyLFfXexV8uSG3d2UksSCYdunG` |
| Light Compressed Token | `cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m` |
| Light System Program | `SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7` |
