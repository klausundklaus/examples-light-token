# Light Token Examples

Light token is a high-performance token standard that reduces the cost of mint and token accounts by 200x.

* All light mint and token accounts are on-chain accounts like SPL, but the light token program sponsors the rent-exemption cost for you.
* Light-token accounts can hold balances from any light, SPL, or Token-2022 mint.
* Light-mint accounts represent a unique mint and optionally can store token-metadata. Functionally equivalent to SPL mints.

### Toolkits

- **[Payments and Wallets](toolkits/payments-and-wallets/)** - All you need for wallet integrations and payment flows. Minimal API differences to SPL.
- **[Streaming Tokens](toolkits/streaming-tokens/)** - Stream mint events using Laserstream

### TypeScript Client

TypeScript examples for light-token-sdk.

- **create-mint** - Create a light-token mint
  - [Action](typescript-client/actions/create-mint.ts) | [Instruction](typescript-client/instructions/create-mint.ts)
- **create-ata** - Create an associated light-token account
  - [Action](typescript-client/actions/create-ata.ts) | [Instruction](typescript-client/instructions/create-ata.ts)
- **load-ata** - Load token accounts from light-token, compressed tokens, SPL/T22 to one unified balance.
  - [Action](typescript-client/actions/load-ata.ts) | [Instruction](typescript-client/instructions/load-ata.ts)
- **mint-to** - Mint tokens to a light-account
  - [Action](typescript-client/actions/mint-to.ts) | [Instruction](typescript-client/instructions/mint-to.ts)
- **transfer-interface** - Transfer between light-token, T22, and SPL accounts
  - [Action](typescript-client/actions/transfer-interface.ts) | [Instruction](typescript-client/instructions/transfer-interface.ts)
- **wrap** - Wrap SPL/T22 to light-token
  - [Action](typescript-client/actions/wrap.ts)
- **unwrap** - Unwrap light-token to SPL/T22
  - [Action](typescript-client/actions/unwrap.ts)
 
## Documentation

Learn more [about to Light-Token here](https://www.zkcompression.com/light-token/welcome).
