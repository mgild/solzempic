# Solzempic

Zero-overhead Solana instruction framework for [Pinocchio](https://github.com/anza-xyz/pinocchio).

## Features

- **Zero runtime overhead** - All abstractions compile away
- **Type-safe account wrappers** - `AccountRef`, `AccountRefMut`, `Signer`, etc.
- **Three-phase instruction pattern** - Build, Validate, Execute
- **Shank IDL generation** - Auto-derive Shank-compatible annotations

## Quick Start

```rust
use solzempic::SolzempicEntrypoint;

#[SolzempicEntrypoint("Your11111111111111111111111111111111111111")]
pub enum MyInstruction {
    Initialize = 0,
    Transfer = 1,
}
```

## Shank IDL Generation

Solzempic provides automatic Shank IDL generation from your instruction struct field types.

### Type to Constraint Mapping

| Field Type | Shank Constraint |
|------------|------------------|
| `Signer<'a>` | signer |
| `MutSigner<'a>` | signer, writable |
| `AccountRefMut<'a, T>` | writable |
| `AccountRef<'a, T>` | (readonly) |
| `TokenAccountRefMut<'a>` | writable |
| `TokenAccountRef<'a>` | (readonly) |
| `Mint<'a>` | (readonly) |
| `Writable<'a>` | writable |
| `ReadOnly<'a>` | (readonly) |
| `SystemProgram<'a>` | program |
| `TokenProgram<'a>` | program |
| `AtaProgram<'a>` | program |
| `ShardRefContext<'a, T>` | 3 writable accounts |

### Usage

1. **Annotate your instruction struct:**

The `#[instruction]` macro is polymorphic - it works on both structs (for Shank metadata) and impl blocks (for trait implementations):

```rust
use solzempic::instruction;

#[instruction]  // On struct: generates Shank account metadata
pub struct Transfer<'a> {
    pub source: TokenAccountRefMut<'a>,
    pub destination: TokenAccountRefMut<'a>,
    pub owner: Signer<'a>,
    pub token_program: TokenProgram<'a>,
}

#[instruction(TransferParams)]  // On impl: generates Instruction trait impl
impl<'a> Transfer<'a> {
    fn build(accounts: &'a [AccountView], _params: &TransferParams) -> Result<Self, ProgramError> {
        // ...
    }
    fn validate(&self, _program_id: &Address, _params: &TransferParams) -> ProgramResult { Ok(()) }
    fn execute(&self, _program_id: &Address, _params: &TransferParams) -> ProgramResult { Ok(()) }
}
```

2. **Access the generated Shank attributes:**

The `#[instruction]` macro on structs generates:
- `Transfer::NUM_ACCOUNTS` - Number of accounts required
- `Transfer::SHANK_ACCOUNTS` - Array of `ShankAccountMeta` structs
- `Transfer::shank_accounts()` - Returns Shank attribute strings for copy-paste

Example output from `shank_accounts()`:
```rust
#[account(0, writable, name="source")]
#[account(1, writable, name="destination")]
#[account(2, signer, name="owner")]
#[account(3, name="token_program")]
```

3. **Enable Shank derives (optional):**

Add the `shank` feature to your Cargo.toml:

```toml
[dependencies]
solzempic = { version = "0.1", features = ["shank"] }
shank = "0.4"
```

With the `shank` feature enabled:
- `#[SolzempicEntrypoint]` adds `#[derive(ShankInstruction)]` to the enum
- `#[derive(Account)]` adds `#[derive(ShankAccount)]` to account structs

### Testing IDL Generation

**1. Verify generated metadata in tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_shank_metadata() {
        // Check account count
        assert_eq!(Transfer::NUM_ACCOUNTS, 4);

        // Verify specific account constraints
        let accounts = &Transfer::SHANK_ACCOUNTS;

        assert_eq!(accounts[0].name, "source");
        assert!(accounts[0].is_writable);
        assert!(!accounts[0].is_signer);

        assert_eq!(accounts[2].name, "owner");
        assert!(accounts[2].is_signer);

        assert_eq!(accounts[3].name, "token_program");
        assert!(accounts[3].is_program);
    }

    #[test]
    fn test_shank_attribute_output() {
        // Print Shank attributes for manual verification
        println!("{}", Transfer::shank_accounts());

        // Verify the output contains expected attributes
        let attrs = Transfer::shank_accounts();
        assert!(attrs.contains("writable"));
        assert!(attrs.contains("signer"));
    }
}
```

**2. Print Shank attributes for your instruction enum:**

The `shank_accounts()` method returns copy-paste ready attributes:

```rust
fn main() {
    println!("Transfer accounts:");
    println!("{}", Transfer::shank_accounts());
}
```

Output:
```
#[account(0, writable, name="source")]
#[account(1, writable, name="destination")]
#[account(2, signer, name="owner")]
#[account(3, name="token_program")]
```

### Building the IDL

1. Install shank CLI:
```bash
cargo install shank-cli
```

2. Generate IDL from your program:
```bash
shank idl -o idl.json -p target/deploy/your_program.so
```

Or use the shank Rust API to extract the IDL programmatically.

## Account Wrapper Types

### Signers
- `Signer<'a>` - Validated signer account (readonly by default)
- `MutSigner<'a>` - Validated signer + writable (for accounts receiving lamports)
- `Payer<'a>` - Type alias for `Signer` (semantic: pays for transactions)

### Typed Accounts
- `AccountRef<'a, T>` - Read-only typed account with ownership validation
- `AccountRefMut<'a, T>` - Writable typed account with ownership validation

### Raw Account Wrappers
- `Writable<'a>` - Explicit writable wrapper for `&AccountView`
- `ReadOnly<'a>` - Explicit readonly wrapper for `&AccountView`

### Token Accounts
- `TokenAccountRefMut<'a>` - Writable SPL token account
- `TokenAccountRef<'a>` - Read-only SPL token account
- `Mint<'a>` - SPL token mint (readonly)
- `TokenAccount<'a>` - Token account wrapper

### Programs
- `SystemProgram<'a>` - Validated system program
- `TokenProgram<'a>` - Validated SPL Token program
- `AtaProgram<'a>` - Validated Associated Token Account program

### Sharded Data
- `ShardRefContext<'a, T>` - Context for sharded orderbooks (holds left, current, right shards)

## License

MIT
