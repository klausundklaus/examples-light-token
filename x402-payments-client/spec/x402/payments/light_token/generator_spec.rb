# frozen_string_literal: true

RSpec.describe X402::Payments::LightToken::Generator do
  let(:test_wallet) { "EYNQARNg9gZTtj1xMMrHK7dRFAkVjAAMubxaH7Do8d9Y" }
  let(:test_private_key) { "5qWMxUb8XGRz7CzTpBLCvSGXoYZeZk4iUwHZJHWqA7eBVPrCwAmRqpMgAQHvqpCjdWNj1iLpCmvPXd8KQgqFPYdC" }
  let(:test_resource) { "http://localhost:3000/api/weather" }
  let(:mock_blockhash) { "GWWy2aAev5X3TMRVwdw8W2KMN3dyVrHrQMZukGTf9R1A" }

  let(:mock_client) do
    instance_double(SolanaRuby::HttpClient).tap do |client|
      allow(client).to receive(:get_latest_blockhash).and_return({ "blockhash" => mock_blockhash })
    end
  end

  before do
    X402::Payments.configure do |config|
      config.default_pay_to = test_wallet
      config.private_key = test_private_key
      config.chain = "solana-devnet"
      config.use_light_token = true
      config.max_timeout_seconds = 600
    end

    allow(SolanaRuby::HttpClient).to receive(:new).and_return(mock_client)
  end

  after do
    X402::Payments.reset_configuration!
  end

  describe "#generate_header" do
    let(:generator) { described_class.new }

    it "generates a base64-encoded payment header" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        description: "Test payment"
      )

      expect(header).to be_a(String)
      expect(header).not_to be_empty

      decoded = Base64.strict_decode64(header)
      expect(decoded).to be_a(String)
    end

    it "creates valid JSON in the header" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 1
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["x402Version"]).to eq(1)
      expect(json["scheme"]).to eq("exact")
      expect(json["network"]).to eq("solana-devnet")
      expect(json["payload"]).to be_a(Hash)
      expect(json["payload"]["transaction"]).to be_a(String)
    end

    it "converts amount to atomic units correctly" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      # 0.001 USD = 1000 atomic units (6 decimals)
      expect(json["accepted"]["amount"]).to eq("1000")
    end

    it "allows overriding network to solana mainnet" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        network: "solana",
        version: 1
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["network"]).to eq("solana")
    end

    it "allows overriding pay_to recipient" do
      different_recipient = "AnotherPubkeyForTestingPurposes123456789ABCD"

      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        pay_to: different_recipient,
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["accepted"]["payTo"]).to eq(different_recipient)
    end

    it "raises error for non-Solana chains" do
      expect {
        generator.generate_header(
          amount: 0.001,
          resource: test_resource,
          network: "base-sepolia"
        )
      }.to raise_error(X402::Payments::ConfigurationError, /Light Token requires a Solana chain/)
    end

    it "raises error when use_light_token is false" do
      X402::Payments.configuration.use_light_token = false

      expect {
        generator.generate_header(
          amount: 0.001,
          resource: test_resource
        )
      }.to raise_error(X402::Payments::ConfigurationError, /Light Token requires a Solana chain with use_light_token=true/)
    end

    it "raises error when private key is missing" do
      X402::Payments.configuration.private_key = nil

      expect {
        generator.generate_header(amount: 0.001, resource: test_resource)
      }.to raise_error(X402::Payments::ConfigurationError, /private key is required/)
    end

    it "raises error when recipient is missing" do
      X402::Payments.configuration.default_pay_to = nil

      expect {
        generator.generate_header(amount: 0.001, resource: test_resource)
      }.to raise_error(X402::Payments::ConfigurationError, /Recipient address is required/)
    end
  end

  describe "v1 protocol support" do
    let(:generator) { described_class.new }

    it "generates v1 payload structure" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 1
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["x402Version"]).to eq(1)
      expect(json["scheme"]).to eq("exact")
      expect(json["network"]).to eq("solana-devnet")
      expect(json["payload"]["transaction"]).to be_a(String)
    end

    it "uses human-readable network name" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 1
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["network"]).to eq("solana-devnet")
    end
  end

  describe "v2 protocol support" do
    let(:generator) { described_class.new }

    it "generates v2 payload structure" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        description: "Test payment",
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["x402Version"]).to eq(2)
      expect(json["resource"]).to be_a(Hash)
      expect(json["accepted"]).to be_a(Hash)
      expect(json["extensions"]).to eq({})
    end

    it "includes resource object with correct fields" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        description: "Test payment",
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["resource"]["url"]).to eq(test_resource)
      expect(json["resource"]["description"]).to eq("Test payment")
      expect(json["resource"]["mimeType"]).to eq("application/json")
    end

    it "uses CAIP-2 network format for devnet" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      # Light Token uses same Solana CAIP-2 identifiers
      expect(json["accepted"]["network"]).to eq("solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1")
    end

    it "includes accepted object with payment requirements" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["accepted"]["scheme"]).to eq("exact")
      expect(json["accepted"]["amount"]).to eq("1000")
      expect(json["accepted"]["payTo"]).to eq(test_wallet)
      expect(json["accepted"]["maxTimeoutSeconds"]).to eq(600)
      expect(json["accepted"]["asset"]).to eq("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU")
    end

    it "includes feePayer in extra" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["accepted"]["extra"]["feePayer"]).to eq("CKPKJWNdJEqa81x7CkZ14BVPiY6y16Sxs7owznqtWYp5")
    end

    it "includes tokenProgram in extra for CToken" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["accepted"]["extra"]["tokenProgram"]).to eq("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m")
    end

    it "includes transaction in payload" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 2
      )

      decoded = Base64.strict_decode64(header)
      json = JSON.parse(decoded)

      expect(json["payload"]["transaction"]).to be_a(String)
      # Transaction should be base64 encoded
      expect { Base64.strict_decode64(json["payload"]["transaction"]) }.not_to raise_error
    end
  end

  describe "amount conversion" do
    let(:generator) { described_class.new }

    it "converts 0.001 USD to 1000 atomic units" do
      header = generator.generate_header(amount: 0.001, resource: test_resource, version: 2)
      decoded = JSON.parse(Base64.strict_decode64(header))
      expect(decoded["accepted"]["amount"]).to eq("1000")
    end

    it "converts 1 USD to 1000000 atomic units" do
      header = generator.generate_header(amount: 1, resource: test_resource, version: 2)
      decoded = JSON.parse(Base64.strict_decode64(header))
      expect(decoded["accepted"]["amount"]).to eq("1000000")
    end

    it "converts 0.000001 USD to 1 atomic unit" do
      header = generator.generate_header(amount: 0.000001, resource: test_resource, version: 2)
      decoded = JSON.parse(Base64.strict_decode64(header))
      expect(decoded["accepted"]["amount"]).to eq("1")
    end
  end

  describe "CToken ATA derivation" do
    let(:generator) { described_class.new }

    it "derives ATA using cToken program ID" do
      # The CToken ATA derivation uses CTOKEN_PROGRAM_ID for both
      # token program and associated token program
      mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
      owner = "EYNQARNg9gZTtj1xMMrHK7dRFAkVjAAMubxaH7Do8d9Y"

      ata = generator.send(:derive_ctoken_associated_token_address, mint, owner)

      # ATA should be a valid base58 address
      expect(ata).to be_a(String)
      expect(ata.length).to be_between(32, 44)
    end

    it "derives different ATAs for different mints" do
      owner = "EYNQARNg9gZTtj1xMMrHK7dRFAkVjAAMubxaH7Do8d9Y"
      mint1 = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
      mint2 = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"

      ata1 = generator.send(:derive_ctoken_associated_token_address, mint1, owner)
      ata2 = generator.send(:derive_ctoken_associated_token_address, mint2, owner)

      expect(ata1).not_to eq(ata2)
    end

    it "derives different ATAs for different owners" do
      mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
      owner1 = "EYNQARNg9gZTtj1xMMrHK7dRFAkVjAAMubxaH7Do8d9Y"
      owner2 = "GWWy2aAev5X3TMRVwdw8W2KMN3dyVrHrQMZukGTf9R1A"

      ata1 = generator.send(:derive_ctoken_associated_token_address, mint, owner1)
      ata2 = generator.send(:derive_ctoken_associated_token_address, mint, owner2)

      expect(ata1).not_to eq(ata2)
    end
  end

  describe "CToken transfer instruction" do
    let(:generator) { described_class.new }

    it "builds instruction with discriminator 3" do
      instruction = generator.send(
        :build_ctoken_transfer_instruction,
        source_ata: "SourceATA",
        destination_ata: "DestATA",
        owner: "OwnerPubkey",
        amount: 1000
      )

      # First byte should be discriminator 3
      expect(instruction.data[0]).to eq(3)
    end

    it "builds instruction with correct amount encoding" do
      instruction = generator.send(
        :build_ctoken_transfer_instruction,
        source_ata: "SourceATA",
        destination_ata: "DestATA",
        owner: "OwnerPubkey",
        amount: 1000
      )

      # Data should be 9 bytes: 1 byte discriminator + 8 bytes u64 amount
      expect(instruction.data.length).to eq(9)

      # Amount 1000 in little-endian u64
      amount_bytes = instruction.data[1..8]
      decoded_amount = amount_bytes.pack("C*").unpack1("Q<")
      expect(decoded_amount).to eq(1000)
    end

    it "uses cToken program ID" do
      instruction = generator.send(
        :build_ctoken_transfer_instruction,
        source_ata: "SourceATA",
        destination_ata: "DestATA",
        owner: "OwnerPubkey",
        amount: 1000
      )

      expect(instruction.program_id).to eq("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m")
    end

    it "includes correct accounts" do
      instruction = generator.send(
        :build_ctoken_transfer_instruction,
        source_ata: "SourceATA",
        destination_ata: "DestATA",
        owner: "OwnerPubkey",
        amount: 1000
      )

      expect(instruction.keys.length).to eq(3)

      # Source ATA - writable, not signer
      expect(instruction.keys[0][:pubkey]).to eq("SourceATA")
      expect(instruction.keys[0][:is_writable]).to be true
      expect(instruction.keys[0][:is_signer]).to be false

      # Destination ATA - writable, not signer
      expect(instruction.keys[1][:pubkey]).to eq("DestATA")
      expect(instruction.keys[1][:is_writable]).to be true
      expect(instruction.keys[1][:is_signer]).to be false

      # Owner - signer, not writable
      expect(instruction.keys[2][:pubkey]).to eq("OwnerPubkey")
      expect(instruction.keys[2][:is_signer]).to be true
      expect(instruction.keys[2][:is_writable]).to be false
    end
  end

  describe "transaction building" do
    let(:generator) { described_class.new }

    it "fetches recent blockhash from RPC" do
      expect(mock_client).to receive(:get_latest_blockhash)
        .and_return({ "blockhash" => mock_blockhash })

      generator.generate_header(
        amount: 0.001,
        resource: test_resource
      )
    end

    it "builds transaction with correct structure" do
      header = generator.generate_header(
        amount: 0.001,
        resource: test_resource,
        version: 1
      )

      decoded = JSON.parse(Base64.strict_decode64(header))
      expect(decoded["payload"]["transaction"]).to be_a(String)
      # Transaction should be base64 encoded
      expect { Base64.strict_decode64(decoded["payload"]["transaction"]) }.not_to raise_error
    end
  end
end

