// Test setup: creates an SPL mint, funds the sponsor, wraps into Light.
// In production the mint already exists (e.g. USDC) and the sender
// already holds Light tokens. Use wrap.ts to wrap SPL tokens into Light tokens.
// Use unwrap.ts to unwrap Light tokens into SPL tokens.

import { Keypair } from "@solana/web3.js";
import { Rpc } from "@lightprotocol/stateless.js";
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";
import { wrap } from "@lightprotocol/compressed-token/unified";
import {
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccount,
    mintTo,
} from "@solana/spl-token";

export async function setup(
    rpc: Rpc,
    sponsor: Keypair,
    sender: Keypair,
) {
    const decimals = 6;
    const amount = 1_000_000;

    const { mint } = await createMintInterface(
        rpc, sponsor, sponsor, null, decimals,
        undefined, undefined, TOKEN_PROGRAM_ID,
    );

    const sponsorSplAta = await createAssociatedTokenAccount(
        rpc, sponsor, mint, sponsor.publicKey, undefined, TOKEN_PROGRAM_ID,
    );
    await mintTo(rpc, sponsor, mint, sponsorSplAta, sponsor, amount);

    await createAtaInterface(rpc, sponsor, mint, sender.publicKey);
    const senderAta = getAssociatedTokenAddressInterface(mint, sender.publicKey);
    await wrap(rpc, sponsor, sponsorSplAta, senderAta, sponsor, mint, BigInt(amount));

    return { mint, senderAta };
}
