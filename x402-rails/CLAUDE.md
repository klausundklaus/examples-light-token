# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

x402-rails is a Ruby gem that enables Rails applications to accept instant blockchain micropayments using the [x402 payment protocol](https://www.x402.org/). The gem provides a simple one-line integration (`x402_paywall`) to protect API endpoints with cryptocurrency payments (USDC) on EVM chains (Base, Avalanche) and Solana.

**Core Features:**
- One-line paywall integration for any Rails controller action
- Supports x402 protocol v1 (legacy) and v2 (default)
- Multi-chain support: Base, Avalanche, Solana (mainnet and testnets)
- Custom chain and token registration for EVM networks
- Optimistic and non-optimistic settlement modes
- Automatic payment verification via external Facilitator service

## Directory Structure

```
x402-rails/
├── lib/
│   └── x402/
│       ├── rails.rb                    # Main entry point, requires all modules
│       ├── rails/
│       │   ├── controller_extensions.rb  # x402_paywall method for controllers
│       │   ├── railtie.rb               # Rails integration
│       │   ├── version.rb               # Gem version
│       │   └── generators/              # Rails generator for initializer
│       ├── chains.rb                   # Chain configs (Base, Avalanche, Solana)
│       ├── configuration.rb            # Global configuration, custom chains/tokens
│       ├── facilitator_client.rb       # HTTP client for verify/settle API calls
│       ├── payment_payload.rb          # Parses payment data from request headers
│       ├── payment_requirement.rb      # Payment requirement data structure
│       ├── payment_validator.rb        # Validates payments before settlement
│       ├── requirement_generator.rb    # Generates 402 response requirements
│       ├── settlement_response.rb      # Wraps facilitator settlement response
│       └── versions/
│           ├── base.rb                 # Abstract base for version strategies
│           ├── v1.rb                   # x402 v1 protocol (X-PAYMENT headers)
│           └── v2.rb                   # x402 v2 protocol (CAIP-2, PAYMENT-SIGNATURE)
├── spec/                               # RSpec test suite
│   ├── spec_helper.rb
│   └── x402/                           # Unit tests for each module
├── x402-rails.gemspec                  # Gem specification
├── Gemfile                             # Development dependencies
└── Rakefile                            # rake spec to run tests
```

## Development Commands

```bash
# Install dependencies
bundle install

# Run test suite
bundle exec rake spec
# or
bundle exec rspec

# Run specific test file
bundle exec rspec spec/x402/configuration_spec.rb

# Build the gem
bundle exec rake build

# Install locally
bundle exec rake install
```

## Architecture

### Payment Flow

```
┌──────────┐      ┌──────────────┐      ┌─────────────┐
│  Client  │─────▶│  Rails App   │─────▶│ Facilitator │
│          │      │ (x402_paywall)│      │ (x402.org)  │
└──────────┘      └──────────────┘      └─────────────┘
     │                   │                     │
     │                   │                     ▼
     │                   │              ┌──────────────┐
     │                   │              │  Blockchain  │
     │                   │              │  (Base/Sol)  │
     └───────────────────┴──────────────┴──────────────┘
```

1. Client requests protected endpoint without payment header
2. Server returns 402 Payment Required with payment requirements
3. Client submits payment via blockchain, includes proof in header
4. Server validates payment via Facilitator, serves content
5. After response (optimistic) or before (non-optimistic), settlement occurs

### Key Components

**ControllerExtensions** (`lib/x402/rails/controller_extensions.rb`)
- Provides `x402_paywall(amount:, ...)` method for controllers
- Handles payment header extraction, validation, and 402 responses
- Manages optimistic vs non-optimistic settlement timing

**Versions** (`lib/x402/versions/`)
- Strategy pattern for v1 vs v2 protocol differences
- V1: Simple network names, `X-PAYMENT` header, body-only requirements
- V2: CAIP-2 network IDs, `PAYMENT-SIGNATURE` header, `PAYMENT-REQUIRED` header

**Chains** (`lib/x402/chains.rb`)
- Built-in chain configs: base, base-sepolia, avalanche, avalanche-fuji, solana, solana-devnet
- CAIP-2 network identifier mappings
- Token address and decimal configurations

**FacilitatorClient** (`lib/x402/facilitator_client.rb`)
- HTTP client using Faraday
- `verify()`: Validates payment signature and parameters
- `settle()`: Settles payment on blockchain

**PaymentValidator** (`lib/x402/payment_validator.rb`)
- Local validation (scheme, network, recipient, amount for EVM)
- Delegates cryptographic verification to Facilitator

### Protocol Version Differences

| Feature | V1 | V2 (Default) |
|---------|-----|--------------|
| Network format | `base-sepolia` | `eip155:84532` (CAIP-2) |
| Payment header | `X-PAYMENT` | `PAYMENT-SIGNATURE` |
| Response header | `X-PAYMENT-RESPONSE` | `PAYMENT-RESPONSE` |
| Requirement delivery | Body only | `PAYMENT-REQUIRED` header + body |
| Amount field | `maxAmountRequired` | `amount` |

## Configuration

Global configuration via initializer:

```ruby
X402.configure do |config|
  config.wallet_address = ENV['X402_WALLET_ADDRESS']  # Required
  config.facilitator = "https://x402.org/facilitator"
  config.chain = "base-sepolia"
  config.currency = "USDC"
  config.optimistic = true   # Settle after response
  config.version = 2         # Protocol version
end
```

Custom chain/token registration (EVM only):

```ruby
config.register_chain(name: "polygon", chain_id: 137, standard: "eip155")
config.register_token(chain: "polygon", symbol: "USDC", address: "0x...", decimals: 6, name: "USD Coin")
```

Multi-chain acceptance:

```ruby
config.accept(chain: "base-sepolia", currency: "USDC")
config.accept(chain: "avalanche-fuji", currency: "USDC")
```

## Error Types

- `X402::ConfigurationError` - Invalid or missing configuration
- `X402::InvalidPaymentError` - Malformed or invalid payment payload
- `X402::FacilitatorError` - Communication issues with Facilitator service

## Testing Notes

- Tests use RSpec with WebMock for HTTP stubbing
- SimpleCov generates coverage reports in `coverage/`
- Controller extensions are tested with mock request/response objects
- Facilitator interactions should be stubbed in tests

## Code Patterns

### Adding a Controller Paywall

```ruby
def show
  x402_paywall(amount: 0.001)
  return if performed?  # Important: check if 402 was rendered
  
  # Access payment info
  payer = request.env['x402.payment'][:payer]
  render json: { data: "premium content" }
end
```

### Version Strategy Pattern

All version-specific behavior is encapsulated in version strategy classes:

```ruby
version_strategy = X402::Versions.for(protocol_version)
header_name = version_strategy.payment_header_name
formatted_network = version_strategy.format_network("base-sepolia")
```

## Dependencies

- **rails** >= 7.0.0 - Rails integration
- **faraday** ~> 2.0 - HTTP client for Facilitator API
- **faraday-follow_redirects** ~> 0.3 - Redirect handling

Development:
- **rspec** / **rspec-rails** - Testing framework
- **webmock** - HTTP request stubbing
- **vcr** - HTTP interaction recording
- **simplecov** - Code coverage

