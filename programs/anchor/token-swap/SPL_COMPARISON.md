# Repository Comparison: solana-program-examples vs light-token-escrow-fixes

This document compares the standard Solana Program Examples (SPL) AMM token-swap with the Light Token implementation, showing exact code differences section by section.

---

## 1. Program Entry Point

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/lib.rs
use anchor_lang::prelude::*;

declare_id!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

#[program]
pub mod swap_example {
    pub use super::instructions::*;
    use super::*;

    pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
        instructions::create_amm(ctx, id, fee)
    }

    pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
        instructions::create_pool(ctx)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        instructions::deposit_liquidity(ctx, amount_a, amount_b)
    }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
        instructions::withdraw_liquidity(ctx, amount)
    }

    pub fn swap_exact_tokens_for_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        instructions::swap_exact_tokens_for_tokens(ctx, swap_a, input_amount, min_output_amount)
    }
}
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/lib.rs
use anchor_lang::prelude::*;
use light_token::anchor::{derive_light_cpi_signer, light_program, CpiSigner};

declare_id!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

// NEW: CPI signer for Light Protocol operations
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

#[light_program]  // NEW: Light Protocol macro
#[allow(deprecated)]
#[program]
pub mod swap_example {
    pub use super::instructions::*;
    use super::*;

    pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
        instructions::create_amm(ctx, id, fee)
    }

    // Parameters wrapped in struct for proof data
    pub fn create_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePool<'info>>,
        params: CreatePoolParams,
    ) -> Result<()> {
        instructions::create_pool(ctx, params)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        instructions::deposit_liquidity(ctx, amount_a, amount_b)
    }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
        instructions::withdraw_liquidity(ctx, amount)
    }

    pub fn swap_exact_tokens_for_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        instructions::swap_exact_tokens_for_tokens(ctx, swap_a, input_amount, min_output_amount)
    }

    // NEW: Create pool with Light LP mint for fully rent-free operations
    pub fn create_pool_light_lp<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePoolLightLp<'info>>,
        params: CreatePoolLightLpParams,
    ) -> Result<()> {
        instructions::create_pool_light_lp(ctx, params)
    }
}
```

**Key Differences:**

| Aspect | SPL | Light |
| ------ | --- | ----- |
| Macro | `#[program]` | `#[light_program]` + `#[program]` |
| CPI Signer | None | `derive_light_cpi_signer!` constant |
| create_pool params | None | `CreatePoolParams` with proof data |
| Instructions | 5 | 6 (adds `create_pool_light_lp`) |

---

## 2. Instruction Parameters

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/lib.rs:20-22
// No parameters for create_pool
pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
    instructions::create_pool(ctx)
}
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/instructions/create_pool.rs:12-17
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePoolParams {
    pub create_accounts_proof: CreateAccountsProof,  // ZK proof for account creation
    pub pool_account_a_bump: u8,                     // Pre-computed bump for vault A
    pub pool_account_b_bump: u8,                     // Pre-computed bump for vault B
}

// programs/anchor/token-swap/src/instructions/create_pool_light_lp.rs:14-21
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePoolLightLpParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub pool_account_a_bump: u8,
    pub pool_account_b_bump: u8,
    pub lp_mint_signer_bump: u8,      // For Light LP mint derivation
    pub pool_authority_bump: u8,       // For pool authority PDA
}
```

**Key Differences:**

- Light requires `CreateAccountsProof` for compressed account creation
- Light pre-computes bump seeds client-side
- `CreatePoolLightLpParams` adds bumps for Light LP mint operations

---

## 3. State Account Definition

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/state.rs
use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Amm {
    pub id: Pubkey,
    pub admin: Pubkey,
    pub fee: u16,
}

impl Amm {
    pub const LEN: usize = 8 + 32 + 32 + 2;
}

#[account]
#[derive(Default)]
pub struct Pool {
    pub amm: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
}

impl Pool {
    pub const LEN: usize = 8 + 32 + 32 + 32;
}
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/state.rs
use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Amm {
    pub id: Pubkey,
    pub admin: Pubkey,
    pub fee: u16,
}

impl Amm {
    pub const LEN: usize = 8 + 32 + 32 + 2;
}

#[account]
#[derive(Default)]
pub struct Pool {
    pub amm: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub lp_supply: u64,  // NEW: Tracked on-chain for Light LP mint compatibility
}

impl Pool {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 8;
}
```

**Key Differences:**

| Aspect | SPL | Light |
| ------ | --- | ----- |
| Pool size | 104 bytes | 112 bytes (+8) |
| LP supply tracking | Via mint.supply | `lp_supply` field for Light mints |

The `lp_supply` field is needed because Light mints don't store supply on-chain like SPL mints.

---

## 4. CreatePool Account Constraints

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/instructions/create_pool.rs:21-99
#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(seeds = [amm.id.as_ref()], bump)]
    pub amm: Box<Account<'info, Amm>>,

    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [amm.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: Read only authority
    #[account(
        seeds = [amm.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref(), AUTHORITY_SEED],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [amm.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref(), LIQUIDITY_SEED],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority,
    )]
    pub mint_liquidity: Box<Account<'info, Mint>>,

    pub mint_a: Box<Account<'info, Mint>>,
    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_b: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/instructions/create_pool.rs:29-135
#[derive(Accounts, LightAccounts)]  // NEW: LightAccounts derive
#[instruction(params: CreatePoolParams)]
pub struct CreatePool<'info> {
    #[account(seeds = [amm.id.as_ref()], bump)]
    pub amm: Box<Account<'info, Amm>>,

    #[account(
        init,
        payer = fee_payer,
        space = Pool::LEN,
        seeds = [amm.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: PDA verified by seeds constraint
    #[account(
        seeds = [amm.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref(), AUTHORITY_SEED],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        seeds = [amm.key().as_ref(), mint_a.key().as_ref(), mint_b.key().as_ref(), LIQUIDITY_SEED],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority,
        mint::token_program = liquidity_token_program,
    )]
    pub mint_liquidity: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: PDA verified by seeds constraint
    #[account(mut, seeds = [POOL_ACCOUNT_A_SEED, pool.key().as_ref()], bump)]
    #[light_account(init,
        token::authority = [POOL_ACCOUNT_A_SEED, self.pool.key()],
        token::mint = mint_a,
        token::owner = pool_authority,
        token::bump = params.pool_account_a_bump
    )]
    pub pool_account_a: UncheckedAccount<'info>,

    /// CHECK: PDA verified by seeds constraint
    #[account(mut, seeds = [POOL_ACCOUNT_B_SEED, pool.key().as_ref()], bump)]
    #[light_account(init,
        token::authority = [POOL_ACCOUNT_B_SEED, self.pool.key()],
        token::mint = mint_b,
        token::owner = pool_authority,
        token::bump = params.pool_account_b_bump
    )]
    pub pool_account_b: UncheckedAccount<'info>,

    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub liquidity_token_program: Interface<'info, TokenInterface>,
    pub light_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    // ========== Light Protocol Accounts ==========
    /// CHECK: Validated by address constraint
    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    /// CHECK: Validated by address constraint
    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,
}
```

**Key Differences:**

| Aspect | SPL | Light |
| ------ | --- | ----- |
| Derive | `Accounts` | `Accounts` + `LightAccounts` |
| Vault type | `Account<TokenAccount>` | `UncheckedAccount` with `#[light_account]` |
| Vault seeds | ATA derivation | Custom `POOL_ACCOUNT_A_SEED`, `POOL_ACCOUNT_B_SEED` |
| Token program | Single `Token` | Three: `token_program`, `liquidity_token_program`, `light_token_program` |
| Extra accounts | 3 (system, token, ata) | 6+ (+ Light protocol accounts) |

---

## 5. Token Transfer Logic

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/instructions/deposit_liquidity.rs:78-100
// Transfer tokens to the pool
token::transfer(
    CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.depositor_account_a.to_account_info(),
            to: ctx.accounts.pool_account_a.to_account_info(),
            authority: ctx.accounts.depositor.to_account_info(),
        },
    ),
    amount_a,
)?;
token::transfer(
    CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.depositor_account_b.to_account_info(),
            to: ctx.accounts.pool_account_b.to_account_info(),
            authority: ctx.accounts.depositor.to_account_info(),
        },
    ),
    amount_b,
)?;
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/instructions/shared.rs:5-81
pub struct SplInterfaceConfig<'info> {
    pub mint: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub spl_interface_pda: AccountInfo<'info>,
    pub spl_interface_pda_bump: u8,
}

fn is_light_account(account: &AccountInfo) -> bool {
    account.owner == &Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID)
}

pub fn transfer_tokens<'info>(
    amount: u64,
    decimals: u8,
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    payer: AccountInfo<'info>,
    light_token_cpi_authority: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    signer_seeds: Option<&[&[u8]]>,
    spl_interface: Option<SplInterfaceConfig<'info>>,
) -> Result<()> {
    let is_light_to_light = is_light_account(&from) && is_light_account(&to);

    if is_light_to_light {
        // Pure Light-to-Light transfer
        let cpi = TransferCheckedCpi {
            source: from,
            mint,
            destination: to,
            amount,
            decimals,
            authority,
            system_program,
            max_top_up: Some(0),
            fee_payer: Some(payer),
        };

        if let Some(seeds) = signer_seeds {
            cpi.invoke_signed(&[seeds])
        } else {
            cpi.invoke()
        }
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e).into())
    } else {
        // SPL<->Light transfer via interface
        let mut cpi = TransferInterfaceCpi::new(
            amount,
            decimals,
            from,
            to,
            authority,
            payer,
            light_token_cpi_authority,
            system_program,
        );

        if let Some(spl) = spl_interface {
            cpi = cpi
                .with_spl_interface(
                    Some(spl.mint),
                    Some(spl.spl_token_program),
                    Some(spl.spl_interface_pda),
                    Some(spl.spl_interface_pda_bump),
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        }

        if let Some(seeds) = signer_seeds {
            cpi.invoke_signed(&[seeds])
        } else {
            cpi.invoke()
        }
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e).into())
    }
}
```

**Key Differences:**

| Aspect | SPL | Light |
| ------ | --- | ----- |
| Function params | 4 typed params | 11 params including Light infra |
| Transfer type | Single `token::transfer` | Conditional: `TransferCheckedCpi` or `TransferInterfaceCpi` |
| Account detection | N/A | `is_light_account()` checks owner |
| Cross-protocol | Not supported | `SplInterfaceConfig` for SPL<->Light |
| CPI pattern | `CpiContext::new()` | Direct struct construction + `invoke()` |

---

## 6. LP Token Minting/Burning

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/instructions/deposit_liquidity.rs:102-123
// Mint the liquidity to user
let authority_bump = ctx.bumps.pool_authority;
let authority_seeds = &[
    &ctx.accounts.pool.amm.to_bytes(),
    &ctx.accounts.mint_a.key().to_bytes(),
    &ctx.accounts.mint_b.key().to_bytes(),
    AUTHORITY_SEED,
    &[authority_bump],
];
let signer_seeds = &[&authority_seeds[..]];
token::mint_to(
    CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.mint_liquidity.to_account_info(),
            to: ctx.accounts.depositor_account_liquidity.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        },
        signer_seeds,
    ),
    liquidity,
)?;

// tokens/token-swap/anchor/programs/token-swap/src/instructions/withdraw_liquidity.rs:69-81
// Burn the liquidity tokens
token::burn(
    CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.mint_liquidity.to_account_info(),
            from: ctx.accounts.depositor_account_liquidity.to_account_info(),
            authority: ctx.accounts.depositor.to_account_info(),
        },
    ),
    amount,
)?;
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/instructions/deposit_liquidity.rs:140-167
let is_light_lp = ctx.accounts.liquidity_token_program.key().to_bytes() == LIGHT_TOKEN_PROGRAM_ID;

if is_light_lp {
    // Light LP mint
    MintToCpi {
        mint: ctx.accounts.mint_liquidity.to_account_info(),
        destination: ctx.accounts.depositor_account_liquidity.to_account_info(),
        amount: liquidity,
        authority: ctx.accounts.pool_authority.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        max_top_up: None,
        fee_payer: Some(ctx.accounts.payer.to_account_info()),
    }
    .invoke_signed(&[authority_seeds])?;
} else {
    // SPL/T22 LP mint
    let signer_seeds = &[authority_seeds];
    light_anchor_spl::token_interface::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.liquidity_token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_liquidity.to_account_info(),
                to: ctx.accounts.depositor_account_liquidity.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        liquidity,
    )?;
}

// programs/anchor/token-swap/src/instructions/withdraw_liquidity.rs:100-123
if is_light_lp {
    BurnCpi {
        source: ctx.accounts.depositor_account_liquidity.to_account_info(),
        mint: ctx.accounts.mint_liquidity.to_account_info(),
        amount,
        authority: ctx.accounts.depositor.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        max_top_up: None,
        fee_payer: None,
    }
    .invoke()?;
} else {
    token_interface::burn(
        CpiContext::new(
            ctx.accounts.liquidity_token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint_liquidity.to_account_info(),
                from: ctx.accounts.depositor_account_liquidity.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount,
    )?;
}
```

**Key Differences:**

| Aspect | SPL | Light |
| ------ | --- | ----- |
| Mint CPI | Single `token::mint_to` | Conditional: `MintToCpi` (Light) or `mint_to` (SPL/T22) |
| Burn CPI | Single `token::burn` | Conditional: `BurnCpi` (Light) or `burn` (SPL/T22) |
| LP type detection | N/A | Check `liquidity_token_program` against `LIGHT_TOKEN_PROGRAM_ID` |
| LP supply tracking | Via mint.supply | Manual `pool.lp_supply` field updates |

---

## 7. Balance Reading

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/instructions/deposit_liquidity.rs:32-35
let pool_a = &ctx.accounts.pool_account_a;
let pool_b = &ctx.accounts.pool_account_b;
let pool_creation = pool_a.amount == 0 && pool_b.amount == 0;

// tokens/token-swap/anchor/programs/token-swap/src/instructions/swap_exact_tokens_for_tokens.rs:135-136
// Reload accounts because of the CPIs
ctx.accounts.pool_account_a.reload()?;
ctx.accounts.pool_account_b.reload()?;
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/instructions/deposit_liquidity.rs:23-26
let pool_a_balance = get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
    .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
let pool_b_balance = get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
    .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;

// programs/anchor/token-swap/src/instructions/swap_exact_tokens_for_tokens.rs:174-179
let new_pool_a_balance =
    get_token_account_balance(&ctx.accounts.pool_account_a.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
let new_pool_b_balance =
    get_token_account_balance(&ctx.accounts.pool_account_b.to_account_info())
        .map_err(|_| anchor_lang::prelude::ProgramError::InvalidAccountData)?;
```

**Key Differences:**

| Aspect | SPL | Light |
| ------ | --- | ----- |
| Balance access | Direct `.amount` field | `get_token_account_balance()` helper |
| Account type | Typed `InterfaceAccount<TokenAccount>` | `UncheckedAccount` (Light-aware parsing) |
| Refresh method | `.reload()` | Re-call `get_token_account_balance()` |

---

## 8. Constants

### SPL (solana-program-examples)

```rust
// tokens/token-swap/anchor/programs/token-swap/src/constants.rs
use anchor_lang::prelude::*;

#[constant]
pub const MINIMUM_LIQUIDITY: u64 = 100;

#[constant]
pub const AUTHORITY_SEED: &[u8] = b"authority";

#[constant]
pub const LIQUIDITY_SEED: &[u8] = b"liquidity";
```

### Light (light-token-escrow-fixes)

```rust
// programs/anchor/token-swap/src/constants.rs
use anchor_lang::prelude::*;

#[constant]
pub const MINIMUM_LIQUIDITY: u64 = 100;

#[constant]
pub const AUTHORITY_SEED: &[u8] = b"authority";

#[constant]
pub const LIQUIDITY_SEED: &[u8] = b"liquidity";

// NEW: Pool vault seeds (Light token accounts use custom PDAs, not ATAs)
#[constant]
pub const POOL_ACCOUNT_A_SEED: &[u8] = b"pool_a";

#[constant]
pub const POOL_ACCOUNT_B_SEED: &[u8] = b"pool_b";

// NEW: LP mint signer seed for FullLight config
#[constant]
pub const LP_MINT_SIGNER_SEED: &[u8] = b"lp_mint_signer";
```

**Key Differences:**

- SPL: 3 constants
- Light: 6 constants (adds `POOL_ACCOUNT_A_SEED`, `POOL_ACCOUNT_B_SEED`, `LP_MINT_SIGNER_SEED`)
- Light vaults use custom PDA derivation instead of standard ATAs

---

## 9. Dependencies (Cargo.toml)

### SPL (solana-program-examples)

```toml
# tokens/token-swap/anchor/programs/token-swap/Cargo.toml
[dependencies]
anchor-lang = "0.32.1"
anchor-spl = "0.32.1"
fixed = "1.20.0"

[dev-dependencies]
# None - tests are in TypeScript
```

### Light (light-token-escrow-fixes)

```toml
# programs/anchor/token-swap/Cargo.toml
[dependencies]
anchor-lang.workspace = true           # 0.31.1

# Light Protocol core
light-sdk = { workspace = true, features = [
    "anchor", "anchor-discriminator", "idl-build", "cpi-context", "v2"
] }                                    # 0.19.0
light-token = { workspace = true, features = ["anchor"] }  # 0.4.0
light-anchor-spl = { workspace = true, features = ["idl-build"] } # 0.31.1

# Math
fixed.workspace = true                 # 1.20.0

# Solana (modular imports)
solana-program.workspace = true        # 2.x
solana-pubkey.workspace = true
solana-account-info.workspace = true
solana-program-error.workspace = true
solana-msg.workspace = true

[dev-dependencies]
light-program-test.workspace = true    # 0.19.0
light-client = { workspace = true, features = ["v2", "anchor"] }  # 0.19.0
spl-token-2022.workspace = true        # 7.x
shared-test-utils.workspace = true     # Local test utilities
tokio.workspace = true                 # 1.43.0
anchor-spl.workspace = true            # 0.31.1
spl-pod.workspace = true               # 0.6.0
```

**Key Differences:**

- SPL: 3 dependencies (anchor-lang, anchor-spl, fixed)
- Light: 10+ dependencies including Light Protocol SDK, token, and test infrastructure
- Light uses workspace dependencies for version consistency
- Light has extensive dev-dependencies for Rust-based testing

---

## 10. Test Structure

### SPL (solana-program-examples)

```
tokens/token-swap/anchor/
└── tests/
    └── swap.test.ts    # Single TypeScript test file
```

### Light (light-token-escrow-fixes)

```
programs/anchor/token-swap/
└── tests/
    ├── common/
    │   └── mod.rs           # Shared test utilities
    ├── user_spl.rs          # SPL mint + SPL ATAs
    ├── user_t22.rs          # T22 mint + T22 ATAs
    ├── user_spl_light.rs    # SPL mint + Light ATAs
    ├── user_t22_light.rs    # T22 mint + Light ATAs
    ├── user_light.rs        # Light mint + Light ATAs (SPL LP)
    └── user_light_full.rs   # Light mint + Light ATAs + Light LP
```

### TokenConfig Enum

```rust
// tests/common/mod.rs:46-59
#[derive(Clone, Copy, Debug)]
pub enum TokenConfig {
    Spl,       // SPL mint + SPL ATAs
    Token2022, // T22 mint + T22 ATAs
    LightSpl,  // SPL mint + Light ATAs (offchain compress)
    LightT22,  // T22 mint + Light ATAs (offchain compress)
    Light,     // Light mint + Light ATAs (SPL LP mint)
    FullLight, // Light mint + Light ATAs + Light LP mint
}
```

### Test Context

```rust
// tests/common/mod.rs:108-137
pub struct AmmTestContext {
    pub program_id: Pubkey,
    pub payer: Keypair,
    pub mint_a_pubkey: Pubkey,
    pub mint_b_pubkey: Pubkey,
    pub amm_pda: Pubkey,
    pub amm_id: Pubkey,
    pub pool_pda: Pubkey,
    pub pool_authority: Pubkey,
    pub pool_authority_bump: u8,
    pub mint_liquidity: Pubkey,
    pub pool_account_a: Pubkey,
    pub pool_account_b: Pubkey,
    pub pool_a_bump: u8,
    pub pool_b_bump: u8,
    pub spl_interface_pda_a: Pubkey,
    pub spl_interface_pda_b: Pubkey,
    pub spl_interface_bump_a: u8,
    pub depositor: Keypair,
    pub depositor_ata_a: Pubkey,
    pub depositor_ata_b: Pubkey,
    pub token_config: TokenConfig,
    pub light_mint_authority_a: Option<Keypair>,
    pub lp_mint_signer: Option<Pubkey>,
    pub lp_mint_signer_bump: u8,
    pub compression_config: Pubkey,
}
```

**Key Differences:**

| Aspect | SPL | Light |
| ------ | --- | ----- |
| Language | TypeScript | Rust |
| Framework | Mocha + Chai | tokio + cargo test-sbf |
| Test files | 1 | 6 (per token combination) |
| Setup | Manual or helpers | `shared-test-utils` crate |
| Parametric | No | Yes (`TokenConfig` enum) |

---

## 11. Additional Light Protocol Infrastructure

These components exist only in the Light version:

### Rent-Free Configuration

```rust
// tests/common/mod.rs:166
// Initialize rent-free config (returns the config PDA)
let compression_config = initialize_rent_free_config(rpc, &payer, &program_id).await;
```

### SPL Interface PDAs

```rust
// tests/common/mod.rs:329-332
// Create SPL interface PDAs for both mints
let spl_interface_result_a =
    create_spl_interface_pda(rpc, &payer, &mint_a_pubkey, config.mint_type(), false).await;
let spl_interface_result_b =
    create_spl_interface_pda(rpc, &payer, &mint_b_pubkey, config.mint_type(), false).await;
```

### Create Pool with Light LP Mint

```rust
// programs/anchor/token-swap/src/instructions/create_pool_light_lp.rs
// Instruction for fully rent-free pool operations
pub fn create_pool_light_lp<'info>(
    ctx: Context<'_, '_, '_, 'info, CreatePoolLightLp<'info>>,
    params: CreatePoolLightLpParams,
) -> Result<()> {
    // 1. Create pool vaults via explicit CPI (not macro)
    CreateTokenAccountCpi::rent_free(/* ... */).invoke()?;

    // 2. Initialize pool state
    let pool = &mut ctx.accounts.pool;
    pool.amm = ctx.accounts.amm.key();
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();
    pool.lp_supply = 0;

    // LP mint created via #[light_account(init, ...)] macro
    Ok(())
}
```

### Pool Authority Funding

```rust
// tests/common/mod.rs:1134-1138
// Fund pool_authority for Light-to-Light transfers (rent top-ups)
if ctx.token_config.uses_light_user_accounts() {
    rpc.airdrop_lamports(&ctx.pool_authority, 1_000_000_000)
        .await
        .expect("Fund pool_authority for rent top-ups");
}
```

---

## Summary

| Category | SPL (solana-program-examples) | Light (light-token-escrow-fixes) |
| -------- | ----------------------------- | -------------------------------- |
| **Purpose** | Educational examples | Production Light Protocol AMM |
| **Account model** | Standard Anchor | Compressed (Light accounts) |
| **Rent** | User pays rent-exempt | Sponsor-based (rent-free) |
| **Token support** | SPL only | SPL + Token-2022 + Light native |
| **Cross-protocol** | No | Yes (SPL<->Light via interface) |
| **LP mint options** | SPL only | SPL, Token-2022, or Light |
| **Pool vaults** | ATAs | Light token PDAs |
| **Dependencies** | 3 crates | 10+ crates |
| **Test language** | TypeScript | Rust |
| **Test coverage** | Single flow | 6 token combinations |
| **Instructions** | 5 | 6 (adds `create_pool_light_lp`) |
| **Complexity** | Lower | Higher (more infrastructure) |
| **Use case** | Learning Solana AMM | Building rent-free DeFi |
