# Light Token Examples
## Quickstart

### [Devnet Quickstart](devnet-quickstart/)

End-to-end example: create a mint, token account, mint tokens, and transfer on devnet.

## Cookbook

Step-by-step recipes for light-token on Devnet/Localnet.

### [Actions](cookbook/actions/)

-   **[create-mint](cookbook/actions/create-mint.ts)** - Create a new light-token mint
-   **[create-ata](cookbook/actions/create-ata.ts)** - Create an associated light-token account
-   **[load-ata](cookbook/actions/load-ata.ts)** - Load a cold token account (compressed) to hot balance (light-token ata)
-   **[mint-to](cookbook/actions/mint-to.ts)** - Mint tokens to a light-account
-   **[transfer-interface](cookbook/actions/transfer-interface.ts)** - Transfer tokens between light-token, T22, and SPL token accounts.
-   **[wrap](cookbook/actions/wrap.ts)** - Wrap SPL to light-token
-   **[unwrap](cookbook/actions/unwrap.ts)** - Unwrap light-token to SPL for off-ramps and legacy integrations

### [Instructions](cookbook/instructions/)

Low-level instruction builders:

-   **[create-mint](cookbook/instructions/create-mint.ts)** - Build create mint instruction
-   **[create-ata](cookbook/instructions/create-ata.ts)** - Build create ATA instruction
-   **[load-ata](cookbook/instructions/load-ata.ts)** - Build load ATA instruction
-   **[mint-to](cookbook/instructions/mint-to.ts)** - Build mint-to instruction
-   **[transfer-interface](cookbook/instructions/transfer-interface.ts)** - Build transfer interface instruction for transfers between light-token, T22, and SPL token accounts.
-   **[wrap](cookbook/instructions/wrap.ts)** - Build wrap instruction
-   **[unwrap](cookbook/instructions/unwrap.ts)** - Build unwrap instruction for off-ramps and legacy integrations

## Toolkits

### [Payments and Wallets](toolkits/payments-and-wallets/)

Examples for wallet integrations and payment flows:

-   **[get-balance](toolkits/payments-and-wallets/get-balance.ts)** - Fetch token balances for light-token accounts
-   **[get-history](toolkits/payments-and-wallets/get-history.ts)** - Fetch transaction history for light-token accounts
-   **[send-and-receive](toolkits/payments-and-wallets/send-and-receive.ts)** - Send and receive light-tokens using the transfer interface
-   **[wrap](toolkits/payments-and-wallets/wrap.ts)** - Wrap SPL tokens to light-token
-   **[unwrap](toolkits/payments-and-wallets/unwrap.ts)** - Unwrap light-token to SPL for off-ramps and legacy integrations

### [Streaming Tokens](toolkits/streaming-tokens/)

Rust program example with test to stream mint events of the Light-Token Program.


## Setup

```bash
npm install @lightprotocol/stateless.js@alpha \
            @lightprotocol/compressed-token@alpha
```

```bash
cp .env.example .env # ...and set RPC_URL
```

## Run

From repo root:

```bash
# quickstart
npm run quickstart

# cookbook (local)
npm run cookbook create-mint:action
npm run cookbook compress:action
# ... see cookbook/package.json for more

# payments
npm run toolkit:payments send-and-receive
```


For local net, install via `npm i -g @lightprotocol/zk-compression-cli@alpha`, then run `light test-validator` in a separate terminal.

## Documentation

Learn more [about to Light-Token here](https://www.zkcompression.com/light-token/welcome).

