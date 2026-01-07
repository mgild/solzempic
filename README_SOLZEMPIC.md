# Solzempic

A lightweight, zero-overhead framework for building Solana programs with [Pinocchio](https://github.com/anza-xyz/pinocchio).

Solzempic provides the structure and safety of a framework without the bloat. It implements the **Action pattern** (Build, Validate, Execute) and provides type-safe account wrappers while maintaining full control over compute unit usage.

## Why Solzempic?

### The Problem with Existing Frameworks

**Anchor** is excellent for getting started, but comes with significant overhead:
- Magic discriminators and automatic deserialization add ~2000+ CUs per instruction
- IDL generation bloats binary size
- Implicit borsh serialization prevents zero-copy optimizations
- Hard to reason about exactly what code is generated

**Vanilla Pinocchio** gives maximum control, but requires writing boilerplate:
- Account validation logic repeated across instructions
- No structured pattern for instruction processing
- Easy to forget security checks

### Solzempic's Approach

Solzempic provides **just enough structure** to eliminate boilerplate while maintaining zero overhead:

```
+------------------+---------------+------------------+
|     Anchor       |   Solzempic   | Vanilla Pinocchio|
+------------------+---------------+------------------+
| High abstraction | Right balance | No abstraction   |
| Hidden costs     | Explicit      | Explicit         |
| Magic macros     | Thin macros   | No macros        |
| ~5000 CU/instr   | ~100 CU/instr | ~100 CU/instr    |
+------------------+---------------+------------------+
```

## Key Features

- **Zero-overhead abstractions**: All wrappers compile to the same code you'd write by hand
- **Action pattern**: Structured Build -> Validate -> Execute flow for every instruction
- **Type-safe account wrappers**: `AccountRef<T>`, `AccountRefMut<T>` with ownership validation
- **Program-specific Framework trait**: Configure your program ID once, use everywhere
- **Validated program accounts**: `SystemProgram`, `TokenProgram`, `Signer` etc. with compile-time guarantees
- **Derive macros**: `#[SolzempicInstruction]` and `#[derive(SolzempicDispatch)]` for ergonomic dispatch
- **`no_std` compatible**: Works in constrained Solana runtime environment

## Architecture Overview

```
                          ┌─────────────────────────────────────────────────────────────┐
                          │                    SOLZEMPIC FRAMEWORK                       │
                          └─────────────────────────────────────────────────────────────┘
                                                       │
                          ┌────────────────────────────┼────────────────────────────┐
                          │                            │                            │
                          ▼                            ▼                            ▼
                   ┌──────────────┐            ┌──────────────┐            ┌──────────────┐
                   │  Framework   │            │    Action    │            │   Account    │
                   │    Trait     │            │   Pattern    │            │   Wrappers   │
                   └──────────────┘            └──────────────┘            └──────────────┘
                          │                            │                            │
                          │                    ┌───────┴───────┐                    │
                          ▼                    ▼               ▼                    ▼
                   ┌──────────────┐    ┌──────────────┐ ┌──────────────┐    ┌──────────────┐
                   │ define_     │    │    build()   │ │  validate()  │    │  AccountRef  │
                   │ framework!  │    │              │ │              │    │ AccountRefMut│
                   │             │    │ Accounts +   │ │ Invariants   │    │ ShardRef-    │
                   │ Creates:    │    │ params from  │ │ PDA checks   │    │ Context      │
                   │ - Solzempic │    │ raw bytes    │ │ Ownership    │    └──────────────┘
                   │ - AccountRef│    └──────────────┘ └──────────────┘            │
                   │ - AccountRef│            │               │                    │
                   │   Mut       │            └───────┬───────┘                    │
                   └──────────────┘                   │                            │
                          │                           ▼                            │
                          │                    ┌──────────────┐                    │
                          │                    │  execute()   │◄───────────────────┘
                          │                    │              │
                          └───────────────────►│ State changes│
                                               │ CPI calls    │
                                               │ Token xfers  │
                                               └──────────────┘

                   ┌─────────────────────────────────────────────────────────────────────┐
                   │                         PROGRAM WRAPPERS                            │
                   ├─────────────────┬─────────────────┬─────────────────┬──────────────┤
                   │  SystemProgram  │   TokenProgram  │    Signer       │   Sysvars    │
                   │  AtaProgram     │   Mint          │    Payer        │   Clock      │
                   │  AltProgram     │   TokenAccount  │                 │   Rent       │
                   │  Lut            │   Vault         │                 │   SlotHashes │
                   └─────────────────┴─────────────────┴─────────────────┴──────────────┘
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
solzempic = { version = "0.1" }
pinocchio = { version = "0.7" }
bytemuck = { version = "1.14", features = ["derive"] }
```

## Quick Start

### 1. Define Your Program Entry Point

In your program's `lib.rs`, use the `#[SolzempicEntrypoint]` attribute:

```rust
#![no_std]

use solzempic::SolzempicEntrypoint;

#[SolzempicEntrypoint("YourProgramId111111111111111111111111111")]
pub enum MyInstruction {
    Initialize = 0,
    Increment = 1,
}
```

This single attribute generates:
- `ID: Pubkey` constant and `id() -> &'static Pubkey` function
- `AccountRef<'a, T>`, `AccountRefMut<'a, T>`, `ShardRefContext<'a, T>` type aliases
- `#[repr(u8)]` on the enum
- `TryFrom<u8>` and dispatch methods
- The program entrypoint

### 2. Define Account Types

```rust
use bytemuck::{Pod, Zeroable};
use solzempic::Loadable;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Counter {
    pub discriminator: [u8; 8],
    pub owner: Pubkey,
    pub count: u64,
}

impl Loadable for Counter {
    const DISCRIMINATOR: AccountType = AccountType::Counter; // Your enum
    const LEN: usize = core::mem::size_of::<Self>();
}
```

### 3. Implement an Instruction

Use the `#[instruction]` attribute on an impl block:

```rust
use solzempic::{instruction, Signer, ValidatedAccount};
use pinocchio::{AccountView, program_error::ProgramError, ProgramResult};
use solana_address::Address;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IncrementParams {
    pub amount: u64,
}

pub struct Increment<'a> {
    pub counter: AccountRefMut<'a, Counter>,
    pub owner: Signer<'a>,
}

#[instruction(IncrementParams)]
impl<'a> Increment<'a> {
    fn build(accounts: &'a [AccountView], _params: &IncrementParams) -> Result<Self, ProgramError> {
        if accounts.len() < 2 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        Ok(Self {
            counter: AccountRefMut::load(&accounts[0])?,
            owner: Signer::wrap(&accounts[1])?,
        })
    }

    fn validate(&self, _program_id: &Address, _params: &IncrementParams) -> ProgramResult {
        // Verify owner matches counter's owner
        if self.owner.key() != &self.counter.get().owner {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(())
    }

    fn execute(&self, _program_id: &Address, params: &IncrementParams) -> ProgramResult {
        self.counter.get_mut().count += params.amount;
        Ok(())
    }
}
```

### 4. Process Instructions

The `#[SolzempicEntrypoint]` macro generates everything needed. Your `process_instruction` is simply:

```rust
fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    MyInstruction::process(program_id, accounts, data)
}
```

## Core Concepts

### The Action Pattern

Every instruction follows the same three-phase pattern:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ACTION LIFECYCLE                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        │                           │                           │
        ▼                           ▼                           ▼
┌───────────────────┐     ┌───────────────────┐     ┌───────────────────┐
│      BUILD        │     │     VALIDATE      │     │     EXECUTE       │
├───────────────────┤     ├───────────────────┤     ├───────────────────┤
│ • Deserialize     │     │ • Check invariants│     │ • Modify state    │
│   parameters      │     │ • Verify PDAs     │     │ • Transfer tokens │
│ • Load accounts   │ ──► │ • Check ownership │ ──► │ • Create accounts │
│ • Wrap programs   │     │ • Validate ranges │     │ • Emit events     │
│ • Early validation│     │ • Business rules  │     │ • CPI calls       │
└───────────────────┘     └───────────────────┘     └───────────────────┘
        │                           │                           │
        │         FAIL FAST         │      PURE CHECKS          │     SIDE EFFECTS
        │    (bad accounts = error) │   (no state changes)      │   (point of no return)
        ▼                           ▼                           ▼
```

**Phase 1: Build**
- Extract parameters from instruction data (zero-copy via `parse_params`)
- Load and validate account types (`AccountRef::load`, `AccountRefMut::load`)
- Wrap program accounts (`Signer::wrap`, `TokenProgram::wrap`)
- Fail fast on structural errors

**Phase 2: Validate**
- Check business logic invariants
- Verify PDA derivations if needed
- Validate numerical ranges and relationships
- No state mutations allowed

**Phase 3: Execute**
- Perform all state changes
- Execute token transfers
- Make CPI calls
- This is the "point of no return"

### Account Wrappers

#### `AccountRef<T>` - Read-Only Access

```rust
pub struct AccountRef<'a, T: Loadable, F: Framework> {
    pub info: &'a AccountInfo,
    data: &'a [u8],
    // ...
}

impl<'a, T: Loadable, F: Framework> AccountRef<'a, T, F> {
    /// Load with full validation (ownership + discriminator)
    pub fn load(info: &'a AccountInfo) -> Result<Self, ProgramError>;

    /// Load without ownership check (for cross-program reads)
    pub fn load_unchecked(info: &'a AccountInfo) -> Result<Self, ProgramError>;

    /// Get typed reference to account data
    pub fn get(&self) -> &T;

    /// Check if account is a PDA with given seeds
    pub fn is_pda(&self, seeds: &[&[u8]]) -> (bool, u8);
}
```

#### `AccountRefMut<T>` - Read-Write Access

```rust
impl<'a, T: Loadable, F: Framework> AccountRefMut<'a, T, F> {
    /// Load with validation (ownership + discriminator + is_writable)
    pub fn load(info: &'a AccountInfo) -> Result<Self, ProgramError>;

    /// Get typed reference
    pub fn get(&self) -> &T;

    /// Get mutable typed reference
    pub fn get_mut(&mut self) -> &mut T;

    /// Reload after CPI (updates internal data pointer)
    pub fn reload(&mut self);
}

impl<'a, T: Initializable, F: Framework> AccountRefMut<'a, T, F> {
    /// Initialize a new account
    pub fn init(info: &'a AccountInfo, params: T::InitParams) -> Result<Self, ProgramError>;

    /// Initialize if uninitialized, otherwise load
    pub fn init_if_needed(info: &'a AccountInfo, params: T::InitParams) -> Result<Self, ProgramError>;

    /// Initialize a PDA account (create via CPI + initialize)
    pub fn init_pda(
        info: &'a AccountInfo,
        payer: &AccountInfo,
        system_program: &AccountInfo,
        seeds: &[&[u8]],
        space: usize,
        params: T::InitParams,
    ) -> Result<Self, ProgramError>;
}
```

#### `ShardRefContext<T>` - Triplet Navigation

For sharded data structures that need access to prev/current/next:

```rust
pub struct ShardRefContext<'a, T: Loadable, F: Framework> {
    pub prev: AccountRefMut<'a, T, F>,
    pub current: AccountRefMut<'a, T, F>,
    pub next: AccountRefMut<'a, T, F>,
}

impl<'a, T: Loadable, F: Framework> ShardRefContext<'a, T, F> {
    pub fn new(prev: &'a AccountInfo, current: &'a AccountInfo, next: &'a AccountInfo) -> Result<Self, ProgramError>;
    pub fn current_mut(&mut self) -> &mut T;
    pub fn all_mut(&mut self) -> (&mut T, &mut T, &mut T);
}
```

### Validated Program Wrappers

All program and sysvar accounts validate their identity on construction:

```rust
// Programs
let system = SystemProgram::wrap(&accounts[0])?;      // Validates key == 11111...
let token = TokenProgram::wrap(&accounts[1])?;        // Validates SPL Token or Token-2022
let ata = AtaProgram::wrap(&accounts[2])?;            // Validates ATA program

// Signers
let signer = Signer::wrap(&accounts[3])?;             // Validates is_signer flag
let payer = Payer::wrap(&accounts[4])?;               // Alias for Signer

// Sysvars
let clock = ClockSysvar::wrap(&accounts[5])?;         // Validates Clock sysvar ID
let rent = RentSysvar::wrap(&accounts[6])?;           // Validates Rent sysvar ID

// Token accounts
let mint = Mint::wrap(&accounts[7])?;                 // Validates token program ownership
let vault = Vault::wrap(&accounts[8], &authority)?;   // Validates ownership + authority
let token_account = TokenAccountRefMut::load(&accounts[9])?;
```

### The `Framework` Trait

The `Framework` trait allows account wrappers to know your program's ID without passing it everywhere:

```rust
pub trait Framework {
    const PROGRAM_ID: Pubkey;
}

// define_framework! generates:
pub struct Solzempic;
impl Framework for Solzempic {
    const PROGRAM_ID: Pubkey = YOUR_PROGRAM_ID;
}

// Which allows:
AccountRefMut::<MyAccount>::load(&account)?  // Automatically checks owner == YOUR_PROGRAM_ID
```

### Derive Macros

#### `#[SolzempicEntrypoint("program_id")]`

The main entrypoint attribute that generates everything needed for your program:

```rust
#[SolzempicEntrypoint("Your11111111111111111111111111111111111111")]
pub enum MyInstruction {
    Initialize = 0,
    Transfer = 1,
    Close = 2,
}

// Generates:
// - pub const ID: Address = ...
// - pub fn id() -> &'static Address
// - pub type AccountRef<'a, T> = ...
// - pub type AccountRefMut<'a, T> = ...
// - pub type ShardRefContext<'a, T> = ...
// - #[repr(u8)] on the enum
// - TryFrom<u8> for MyInstruction
// - MyInstruction::process() dispatch method
// - The program entrypoint
```

#### `#[instruction(ParamsType)]`

Implements the `Instruction` trait on an impl block:

```rust
pub struct Transfer<'a> {
    pub from: AccountRefMut<'a, TokenAccount>,
    pub to: AccountRefMut<'a, TokenAccount>,
    pub authority: Signer<'a>,
}

#[instruction(TransferParams)]
impl<'a> Transfer<'a> {
    fn build(accounts: &'a [AccountView], params: &TransferParams) -> Result<Self, ProgramError> { ... }
    fn validate(&self, program_id: &Address, params: &TransferParams) -> ProgramResult { ... }
    fn execute(&self, program_id: &Address, params: &TransferParams) -> ProgramResult { ... }
}

// Generates InstructionParams and Instruction trait implementations
```

## Comparison to Alternatives

### vs Anchor

| Feature | Anchor | Solzempic |
|---------|--------|-----------|
| CU overhead | ~2000-5000 per instruction | ~100 (just your logic) |
| Binary size | Large (IDL, borsh) | Minimal |
| Account validation | Automatic, opaque | Explicit, transparent |
| Serialization | Borsh (copies data) | Zero-copy bytemuck |
| Learning curve | Low (magic) | Medium (explicit) |
| Debugging | Hard (generated code) | Easy (your code) |
| Flexibility | Constrained | Full control |

### vs Vanilla Pinocchio

| Feature | Vanilla Pinocchio | Solzempic |
|---------|-------------------|-----------|
| Boilerplate | High (repeat validation) | Low (wrappers) |
| Structure | None (DIY) | Action pattern |
| Safety | Manual | Enforced by types |
| Program ID handling | Pass everywhere | Framework trait |
| Learning curve | High | Medium |

## Performance Characteristics

Solzempic adds no runtime overhead beyond what you'd write by hand:

- **Account loading**: Single borrow, no copies
- **Parameter parsing**: Zero-copy pointer cast
- **Discriminator checks**: Single byte comparison
- **Program validation**: Pubkey comparison (32-byte memcmp)

All wrapper methods are `#[inline]` and compile away in release builds.

Typical instruction overhead:
- Parse params: ~10 CUs
- Load AccountRefMut: ~50 CUs (borrow + discriminator check)
- Wrap Signer: ~20 CUs (is_signer check)
- Your business logic: varies

## API Reference

### Macros

| Macro | Purpose |
|-------|---------|
| `#[SolzempicEntrypoint("...")]` | Main entrypoint - generates ID, type aliases, dispatch, and entrypoint |
| `#[instruction(Params)]` | Implements `Instruction` trait on an impl block |
| `define_framework!(ID)` | Alternative: manually define framework type aliases |
| `define_account_types! { ... }` | Define account discriminator enum |

### Account Wrappers

| Type | Purpose |
|------|---------|
| `AccountRef<'a, T>` | Read-only typed account with ownership validation |
| `AccountRefMut<'a, T>` | Writable typed account with ownership + is_writable checks |
| `ShardRefContext<'a, T>` | Prev/current/next triplet for sharded data structures |

### Program Wrappers

| Type | Validates |
|------|-----------|
| `SystemProgram` | Key == System Program |
| `TokenProgram` | Key == SPL Token or Token-2022 |
| `AtaProgram` | Key == Associated Token Program |
| `AltProgram` | Key == Address Lookup Table Program |
| `Lut` | Address Lookup Table account |

### Signer Wrappers

| Type | Validates |
|------|-----------|
| `Signer` | `is_signer` flag is true |
| `Payer` | Alias for `Signer` (semantic clarity) |

### Token Wrappers

| Type | Purpose |
|------|---------|
| `Mint` | SPL Token mint account |
| `Vault` | Token account with authority validation |
| `SolVault` | SOL vault (system-owned) |
| `TokenAccountRefMut` | Writable token account |
| `TokenAccountData` | Token account data struct |

### Sysvar Wrappers

| Type | Sysvar |
|------|--------|
| `ClockSysvar` | Clock (slot, timestamp, epoch) |
| `RentSysvar` | Rent parameters |
| `SlotHashesSysvar` | Recent slot hashes |
| `InstructionsSysvar` | Current transaction instructions |
| `RecentBlockhashesSysvar` | Recent blockhashes |

### Traits

| Trait | Purpose |
|-------|---------|
| `Instruction` | Three-phase pattern: `build()` → `validate()` → `execute()` |
| `InstructionParams` | Associates a params type with an instruction |
| `Framework` | Program-specific configuration (program ID) |
| `Loadable` | POD types with discriminator (from braid-types) |
| `Initializable` | Types that can be initialized (from braid-types) |
| `ValidatedAccount` | Common interface for validated wrappers |

### Utility Functions

| Function | Purpose |
|----------|---------|
| `create_pda_account()` | Create and initialize a PDA via CPI |
| `transfer_lamports()` | Transfer SOL between accounts |
| `rent_exempt_minimum()` | Calculate rent-exempt minimum for size |
| `parse_params::<T>()` | Zero-copy parameter parsing |

### Constants

| Constant | Value |
|----------|-------|
| `SYSTEM_PROGRAM_ID` | System Program |
| `TOKEN_PROGRAM_ID` | SPL Token Program |
| `TOKEN_2022_PROGRAM_ID` | Token-2022 Program |
| `ASSOCIATED_TOKEN_PROGRAM_ID` | ATA Program |
| `ADDRESS_LOOKUP_TABLE_PROGRAM_ID` | ALT Program |
| `CLOCK_SYSVAR_ID` | Clock sysvar |
| `RENT_SYSVAR_ID` | Rent sysvar |
| `SLOT_HASHES_SYSVAR_ID` | SlotHashes sysvar |
| `INSTRUCTIONS_SYSVAR_ID` | Instructions sysvar |
| `RECENT_BLOCKHASHES_SYSVAR_ID` | RecentBlockhashes sysvar |
| `LAMPORTS_PER_BYTE` | Rent cost per byte |
| `MAX_ACCOUNT_SIZE` | Maximum account size (10MB) |

## Error Handling

Solzempic uses Pinocchio's `ProgramError` throughout:

```rust
// Common errors returned by wrappers:
ProgramError::IllegalOwner           // Account not owned by program
ProgramError::InvalidAccountData     // Wrong discriminator or not writable
ProgramError::AccountAlreadyInitialized  // init() on initialized account
ProgramError::IncorrectProgramId     // Wrong program/sysvar ID
ProgramError::MissingRequiredSignature   // Signer check failed
ProgramError::NotEnoughAccountKeys   // Too few accounts passed
ProgramError::InvalidInstructionData // Params too short
```

Define your own errors by implementing `Into<ProgramError>`:

```rust
#[repr(u32)]
pub enum MyError {
    InvalidPrice = 1000,
    OrderNotFound = 1001,
}

impl From<MyError> for ProgramError {
    fn from(e: MyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
```

## Best Practices

1. **Fail fast in build()**: Validate account structure early
2. **Keep validate() pure**: No state changes, only checks
3. **Document account order**: Use comments to specify expected accounts
4. **Use #[inline(always)]**: For hot paths in execute()
5. **Prefer AccountRefMut::init_pda()**: For PDA creation with initialization
6. **Call reload() after CPI**: If you modify accounts via CPI

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure `cargo clippy` passes
5. Submit a pull request

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

Built on top of [Pinocchio](https://github.com/anza-xyz/pinocchio), the minimal Solana program framework.
