# x402-rails Light Token Migration Report

## Summary

Added Light Token (compressed token) support to the x402-rails gem, enabling Rails applications to accept payments using the Light Protocol's CToken program on Solana.

## Changes Made

### 1. Chain Configurations (`lib/x402/chains.rb`)

Added Light Token chains to `CHAINS` constant:

```ruby
"light-token-devnet" => {
  chain_id: 103,
  usdc_address: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
  explorer_url: "https://explorer.solana.com/?cluster=devnet",
  fee_payer: "CKPKJWNdJEqa81x7CkZ14BVPiY6y16Sxs7owznqtWYp5",
  token_program: "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"
},
"light-token" => {
  chain_id: 101,
  usdc_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  explorer_url: "https://explorer.solana.com",
  fee_payer: "CKPKJWNdJEqa81x7CkZ14BVPiY6y16Sxs7owznqtWYp5",
  token_program: "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"
}
```

Added CAIP2 mappings:

```ruby
"light-token-devnet" => "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1"
"light-token" => "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"
```

Added helper methods:

- `light_token_chain?(chain_name)` - Returns true for Light Token chains
- `token_program_for(chain_name)` - Returns the CToken program ID

### 2. Requirement Generator (`lib/x402/requirement_generator.rb`)

Updated `build_extra_data` method to include `tokenProgram` in the extra field for Light Token chains:

```ruby
if X402.light_token_chain?(chain_name)
  fee_payer = fee_payer_override || X402.fee_payer_for(chain_name)
  token_program = X402.token_program_for(chain_name)
  { feePayer: fee_payer, tokenProgram: token_program }.compact
elsif X402.solana_chain?(chain_name)
  # ... existing Solana logic
end
```

### 3. Payment Payload (`lib/x402/payment_payload.rb`)

Added `light_token_chain?` method and updated chain detection:

```ruby
def light_token_chain?
  X402.light_token_chain?(network)
end

def evm_chain?
  !solana_chain? && !light_token_chain?
end

def transaction
  return nil unless solana_chain? || light_token_chain?
  payload&.with_indifferent_access&.[](:transaction)
end
```

### 4. Payment Validator (`lib/x402/payment_validator.rb`)

Added Light Token transaction validation path:

```ruby
elsif payment_payload.light_token_chain?
  unless payment_payload.transaction
    return validation_error("Light Token payment missing transaction payload")
  end
end
```

## Files Modified

| File | Lines Changed | Description |
|------|---------------|-------------|
| `lib/x402/chains.rb` | +30 | Added Light Token chains, CAIP2 mappings, helper methods |
| `lib/x402/requirement_generator.rb` | +5 | Added tokenProgram to extra for Light Token |
| `lib/x402/payment_payload.rb` | +8 | Added light_token_chain? helper, updated evm_chain? |
| `lib/x402/payment_validator.rb` | +5 | Added Light Token validation branch |
| `spec/x402/chains_spec.rb` | +80 | Added Light Token chain specs |
| `spec/x402/payment_validator_spec.rb` | +70 | Added Light Token payment validation specs |
| `spec/x402/requirement_generator_spec.rb` | +60 | Added Light Token requirement generation specs |

## Reference Patterns Used

Based on `examples-light-token/toolkits/payments-and-wallets/comparison-spl-light.md`:

| Light Token Concept | Implementation |
|---------------------|----------------|
| CToken Program ID | `cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m` |
| Same USDC addresses as Solana | Reused from Solana chain configs |
| Same CAIP2 identifiers as Solana | Uses `solana:*` namespace |
| tokenProgram in extra | Included in 402 response |

## Build Result

- All Ruby files pass syntax validation
- Test suite requires system dependencies (libyaml-dev) to run

## Usage

### Configuration

```ruby
X402.configure do |config|
  config.wallet_address = "YourSolanaWalletAddress"
  config.chain = "light-token-devnet"  # or "light-token" for mainnet
  config.currency = "USDC"
end
```

### Controller

```ruby
class Api::PremiumController < ApplicationController
  def show
    x402_paywall(amount: 0.001, chain: "light-token-devnet")
    return if performed?

    render json: { content: "Premium content" }
  end
end
```

### 402 Response

The 402 response will include `tokenProgram` in the extra field:

```json
{
  "x402Version": 2,
  "accepts": [{
    "scheme": "exact",
    "network": "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
    "amount": "1000",
    "asset": "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
    "payTo": "YourSolanaWalletAddress",
    "extra": {
      "feePayer": "CKPKJWNdJEqa81x7CkZ14BVPiY6y16Sxs7owznqtWYp5",
      "tokenProgram": "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"
    }
  }]
}
```

## Design Decisions

### CAIP-2 Reverse Mapping

Light Token chains share the same Solana chain IDs and CAIP-2 identifiers:
- `light-token-devnet` and `solana-devnet` both map to `solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1`
- `light-token` and `solana` both map to `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp`

For reverse lookups (`from_caip2`), standard Solana takes priority. This means:
- `X402.from_caip2("solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1")` returns `"solana-devnet"` (not `"light-token-devnet"`)

Light Token usage should be explicit via chain configuration, not inferred from CAIP-2 identifiers.

## Compatibility

- Compatible with x402-payments gem Light Token generator
- Uses same facilitator endpoint as Solana
- Payment flow identical to Solana, with CToken transactions instead of SPL

