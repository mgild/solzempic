//! # Solzempic - Zero-Overhead Solana Instruction Framework
//!
//! Solzempic is a lightweight framework for building Solana programs with
//! [Pinocchio](https://github.com/anza-xyz/pinocchio). It provides the structure
//! and safety of a framework without the compute unit overhead.
//!
//! ## Philosophy
//!
//! Solzempic occupies the middle ground between Anchor (high-level, magical) and
//! vanilla Pinocchio (low-level, boilerplate-heavy):
//!
//! - **Zero overhead**: All abstractions compile away to the code you'd write by hand
//! - **Explicit over implicit**: You see exactly what's happening, no hidden costs
//! - **Type-safe wrappers**: Catch errors at compile time, not runtime
//! - **Structured patterns**: The Action pattern enforces clean instruction flow
//!
//! ## Core Architecture
//!
//! ### The Action Pattern
//!
//! Every instruction follows three phases:
//!
//! 1. **Parse**: Extract parameters and load accounts from raw bytes
//! 2. **Validate**: Check invariants, verify PDAs, validate business rules
//! 3. **Actuate**: Perform state changes, transfers, and CPI calls
//!
//! This separation ensures validation happens before any mutations, making
//! instructions easier to reason about and audit.
//!
//! ### Account Wrappers
//!
//! - [`AccountRef<T>`]: Read-only typed access with ownership validation
//! - [`AccountRefMut<T>`]: Writable typed access with ownership + is_writable checks
//! - [`ShardRefContext<T>`]: Navigation context for sharded data structures
//!
//! ### Program Wrappers
//!
//! Type-safe wrappers for common Solana programs and sysvars:
//!
//! - [`SystemProgram`], [`TokenProgram`], [`AtaProgram`], [`AltProgram`]
//! - [`Signer`], [`Payer`] - Validated signer accounts
//! - [`Mint`], [`Vault`], [`TokenAccountRefMut`] - SPL Token accounts
//! - [`ClockSysvar`], [`RentSysvar`], [`SlotHashesSysvar`] - Sysvars
//!
//! ## Quick Start
//!
//! ### 1. Define Your Program
//!
//! ```ignore
//! // In lib.rs
//! use solzempic::SolzempicEntrypoint;
//!
//! #[SolzempicEntrypoint("Your11111111111111111111111111111111111111")]
//! pub enum MyInstruction {
//!     Initialize = 0,
//!     Transfer = 1,
//! }
//! ```
//!
//! This single attribute generates:
//! - `ID: Pubkey` constant
//! - `id() -> &'static Pubkey` function
//! - `AccountRef<'a, T>`, `AccountRefMut<'a, T>`, `ShardRefContext<'a, T>` type aliases
//! - `#[repr(u8)]` on the enum
//! - `TryFrom<u8>` and dispatch methods
//!
//! ### 2. Implement an Instruction
//!
//! ```ignore
//! use solzempic::{Signer, ValidatedAccount};
//!
//! #[repr(C)]
//! #[derive(Clone, Copy)]
//! pub struct TransferParams {
//!     pub amount: u64,
//! }
//!
//! pub struct Transfer<'a> {
//!     pub from: AccountRefMut<'a, Wallet>,
//!     pub to: AccountRefMut<'a, Wallet>,
//!     pub owner: Signer<'a>,
//! }
//!
//! #[instruction(TransferParams)]
//! impl<'a> Transfer<'a> {
//!     fn build(accounts: &'a [AccountInfo], _params: &TransferParams) -> Result<Self, ProgramError> {
//!         Ok(Self {
//!             from: AccountRefMut::load(&accounts[0])?,
//!             to: AccountRefMut::load(&accounts[1])?,
//!             owner: Signer::wrap(&accounts[2])?,
//!         })
//!     }
//!
//!     fn validate(&self, _program_id: &Pubkey, params: &TransferParams) -> ProgramResult {
//!         if self.owner.key() != &self.from.get().owner {
//!             return Err(ProgramError::IllegalOwner);
//!         }
//!         if self.from.get().balance < params.amount {
//!             return Err(ProgramError::InsufficientFunds);
//!         }
//!         Ok(())
//!     }
//!
//!     fn execute(&self, _program_id: &Pubkey, params: &TransferParams) -> ProgramResult {
//!         self.from.get_mut().balance -= params.amount;
//!         self.to.get_mut().balance += params.amount;
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ### 3. Set Up Entrypoint
//!
//! ```ignore
//! fn process_instruction(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
//!     MyInstruction::process(program_id, accounts, data)
//! }
//! ```
//!
//! ## Module Organization
//!
//! - [`programs`]: Program and sysvar account wrappers
//! - Account utilities: [`create_pda_account`], [`transfer_lamports`], [`rent_exempt_minimum`]
//!
//! ## Performance
//!
//! Solzempic adds no runtime overhead:
//!
//! - Account loading: Single borrow, no copies (~50 CUs)
//! - Parameter parsing: Zero-copy pointer cast (~10 CUs)
//! - Program validation: 32-byte memcmp (~20 CUs)
//!
//! All wrapper methods are `#[inline]` and compile away in release builds.

#![no_std]

mod account;
pub mod programs;
mod traits;
mod wrappers;

pub use account::{create_pda_account, rent_exempt_minimum, transfer_lamports, LAMPORTS_PER_BYTE, MAX_ACCOUNT_SIZE};

// Re-export programs module items at crate root for convenience
pub use programs::{
    // IDs
    SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID,
    ADDRESS_LOOKUP_TABLE_PROGRAM_ID, CLOCK_SYSVAR_ID, RENT_SYSVAR_ID, SLOT_HASHES_SYSVAR_ID,
    INSTRUCTIONS_SYSVAR_ID, RECENT_BLOCKHASHES_SYSVAR_ID,
    // Traits
    ValidatedAccount,
    // Program wrappers
    SystemProgram, TokenProgram, AtaProgram, AltProgram, Lut,
    // Signer/Payer/MutSigner
    Signer, Payer, MutSigner,
    // Explicit mutability wrappers (for raw AccountView)
    Writable, ReadOnly,
    // Sysvars
    ClockSysvar, RentSysvar, SlotHashesSysvar, InstructionsSysvar, RecentBlockhashesSysvar,
    // Token
    Mint, TokenAccountData, TokenAccountRefMut, Vault, SolVault,
    // Validation
    validate_token_program, validate_system_program, validate_clock_sysvar,
    validate_slot_hashes_sysvar, validate_rent_sysvar,
};
pub use wrappers::{AccountRef, AccountRefMut, AsAccountRef, ShardRefContext};

// Re-export core traits
pub use traits::{check_discriminator, Initializable, Loadable};

// Re-export derive macros
pub use solzempic_macros::{Account, SolzempicEntrypoint, account, instruction};

/// Define an AccountType enum with automatic discriminator values.
///
/// This macro generates a `#[repr(u8)]` enum with the specified variants and values,
/// along with helper methods for discriminator checking.
///
/// # Example
///
/// ```ignore
/// solzempic::define_account_types! {
///     Counter = 1,
///     Market = 2,
///     User = 3,
/// }
/// ```
///
/// Generates:
///
/// ```ignore
/// #[repr(u8)]
/// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// pub enum AccountType {
///     Counter = 1,
///     Market = 2,
///     User = 3,
/// }
///
/// impl AccountType {
///     pub const fn to_bytes(self) -> [u8; 8] {
///         [self as u8, 0, 0, 0, 0, 0, 0, 0]
///     }
///
///     pub fn check(data: &[u8], expected: Self) -> bool {
///         !data.is_empty() && data[0] == expected as u8
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_account_types {
    (
        $(
            $variant:ident = $value:expr
        ),* $(,)?
    ) => {
        /// Account type discriminators for the program.
        #[repr(u8)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum AccountType {
            $(
                $variant = $value,
            )*
        }

        impl AccountType {
            /// Convert the account type to an 8-byte discriminator array.
            ///
            /// The discriminator value is stored in the first byte,
            /// with the remaining bytes zeroed.
            #[inline]
            pub const fn to_bytes(self) -> [u8; 8] {
                [self as u8, 0, 0, 0, 0, 0, 0, 0]
            }

            /// Check if account data has the expected discriminator.
            ///
            /// Returns true if the data is non-empty and the first
            /// byte matches the expected account type.
            #[inline]
            pub fn check(data: &[u8], expected: Self) -> bool {
                !data.is_empty() && data[0] == expected as u8
            }
        }
    };
}

use pinocchio::AccountView;
use pinocchio::error::ProgramError;
use solana_address::Address;
use pinocchio::ProgramResult;

// Re-export address_eq for efficient pubkey comparisons (4 u64 comparisons vs byte-by-byte)
pub use solana_address::address_eq;

/// The Instruction trait defines the three-phase instruction processing pattern.
///
/// Every instruction handler implements this trait to define its behavior:
///
/// 1. **Build**: Extract the instruction context from raw accounts and parameters
/// 2. **Validate**: Check all invariants before making any state changes
/// 3. **Execute**: Perform the actual state mutations and side effects
///
/// # Example
///
/// ```ignore
/// pub struct Transfer<'a> {
///     pub from: AccountRefMut<'a, Account>,
///     pub to: AccountRefMut<'a, Account>,
///     pub authority: Signer<'a>,
/// }
///
/// impl Instruction for Transfer<'_> {
///     type Params = TransferParams;
///
///     fn build(accounts: &[AccountInfo], params: &Self::Params) -> Result<Self, ProgramError> {
///         Ok(Self {
///             from: AccountRefMut::load(&accounts[0])?,
///             to: AccountRefMut::load(&accounts[1])?,
///             authority: Signer::wrap(&accounts[2])?,
///         })
///     }
///
///     fn validate(&self, _program_id: &Pubkey, params: &Self::Params) -> ProgramResult {
///         if self.from.get().balance < params.amount {
///             return Err(ProgramError::InsufficientFunds);
///         }
///         Ok(())
///     }
///
///     fn execute(&self, _program_id: &Pubkey, params: &Self::Params) -> ProgramResult {
///         self.from.get_mut().balance -= params.amount;
///         self.to.get_mut().balance += params.amount;
///         Ok(())
///     }
/// }
/// ```
///
/// # Design Rationale
///
/// The separation into three phases provides several benefits:
///
/// - **Auditability**: Validators can verify all checks happen before mutations
/// - **Testability**: Each phase can be tested in isolation
/// - **Fail-fast**: Structural errors are caught before any state changes
/// - **Clarity**: Clear separation between "what we need" and "what we do"
/// Marker trait that associates a params type with an instruction.
///
/// This allows dispatch to know the params type without a lifetime parameter.
pub trait InstructionParams {
    /// The parameter type for this instruction (must be `Copy` for zero-copy parsing).
    type Params: Copy;
}

/// The Instruction trait defines the three-phase instruction processing pattern.
///
/// Every instruction handler implements this trait to define its behavior:
///
/// 1. **Build**: Extract the instruction context from raw accounts and parameters
/// 2. **Validate**: Check all invariants before making any state changes
/// 3. **Execute**: Perform the actual state mutations and side effects
///
/// # Example
///
/// ```ignore
/// pub struct Transfer<'a> {
///     pub from: AccountRefMut<'a, Account>,
///     pub to: AccountRefMut<'a, Account>,
///     pub authority: Signer<'a>,
/// }
///
/// impl InstructionParams for Transfer<'_> {
///     type Params = TransferParams;
/// }
///
/// impl<'a> Instruction<'a> for Transfer<'a> {
///     fn build(accounts: &'a [AccountInfo], params: &Self::Params) -> Result<Self, ProgramError> {
///         Ok(Self {
///             from: AccountRefMut::load(&accounts[0])?,
///             to: AccountRefMut::load(&accounts[1])?,
///             authority: Signer::wrap(&accounts[2])?,
///         })
///     }
///
///     fn validate(&self, _program_id: &Pubkey, params: &Self::Params) -> ProgramResult {
///         if self.from.get().balance < params.amount {
///             return Err(ProgramError::InsufficientFunds);
///         }
///         Ok(())
///     }
///
///     fn execute(&self, _program_id: &Pubkey, params: &Self::Params) -> ProgramResult {
///         self.from.get_mut().balance -= params.amount;
///         self.to.get_mut().balance += params.amount;
///         Ok(())
///     }
/// }
/// ```
pub trait Instruction<'a>: InstructionParams + Sized {
    /// Build the instruction context from accounts and parameters.
    fn build(accounts: &'a [AccountView], params: &Self::Params) -> Result<Self, ProgramError>;

    /// Validate business logic invariants.
    fn validate(&self, program_id: &Address, params: &Self::Params) -> ProgramResult;

    /// Execute the instruction and perform state changes.
    fn execute(&self, program_id: &Address, params: &Self::Params) -> ProgramResult;

    /// Process the instruction (parse params -> build context -> validate -> execute).
    #[inline(never)]
    fn process(program_id: &Address, accounts: &'a [AccountView], data: &[u8]) -> ProgramResult {
        let params = parse_params::<Self::Params>(data)?;
        let ctx = Self::build(accounts, &params)?;
        ctx.validate(program_id, &params)?;
        ctx.execute(program_id, &params)
    }
}

/// Parse instruction parameters from raw bytes using zero-copy.
///
/// This function performs a zero-copy cast from the instruction data bytes
/// to the parameter type. The type must be `Copy` (which implies it's a
/// plain-old-data type with no interior references).
///
/// # Type Requirements
///
/// - `T` must be `Copy` (POD type)
/// - `T` should be `#[repr(C)]` for predictable layout
/// - `T` should not have padding bytes that could cause UB
///
/// # Errors
///
/// Returns `InvalidInstructionData` if the data slice is shorter than
/// `size_of::<T>()`.
///
/// # Example
///
/// ```ignore
/// #[repr(C)]
/// #[derive(Clone, Copy)]
/// pub struct TransferParams {
///     pub amount: u64,
///     pub memo_len: u8,
/// }
///
/// let params = parse_params::<TransferParams>(data)?;
/// ```
///
/// # Safety
///
/// This uses unsafe pointer casting internally, but is safe because:
/// - We verify the slice has sufficient length
/// - `T: Copy` guarantees no Drop impl or interior mutability
/// - The cast only reads, never writes
#[inline]
pub fn parse_params<T: Copy>(data: &[u8]) -> Result<T, ProgramError> {
    if data.len() < core::mem::size_of::<T>() {
        return Err(ProgramError::InvalidInstructionData);
    }
    // Safety: We've verified the length and T is Copy (POD)
    let ptr = data.as_ptr() as *const T;
    Ok(unsafe { *ptr })
}

/// Trait that defines program-specific configuration for account wrappers.
///
/// The `Framework` trait allows account wrappers ([`AccountRef`], [`AccountRefMut`])
/// to know your program's ID without passing it as a parameter to every method.
///
/// # Usage
///
/// You typically don't implement this trait directly. Instead, use the
/// [`define_framework!`] macro which creates an implementation and type aliases:
///
/// ```ignore
/// pub const ID: Pubkey = pinocchio_pubkey::pubkey!("Your111...");
/// solzempic::define_framework!(ID);
///
/// // Now you can use:
/// // - AccountRef<'a, T> (aliased to AccountRef<'a, T, Solzempic>)
/// // - AccountRefMut<'a, T> (aliased to AccountRefMut<'a, T, Solzempic>)
/// ```
///
/// # How It Works
///
/// The generic parameter `F: Framework` on account wrappers allows the `load()`
/// method to validate that accounts are owned by your program:
///
/// ```ignore
/// impl<'a, T: Loadable, F: Framework> AccountRef<'a, T, F> {
///     pub fn load(info: &'a AccountInfo) -> Result<Self, ProgramError> {
///         if info.owner() != &F::PROGRAM_ID {
///             return Err(ProgramError::IllegalOwner);
///         }
///         // ... rest of loading
///     }
/// }
/// ```
pub trait Framework {
    /// The program ID used for ownership validation.
    ///
    /// When loading accounts with [`AccountRef::load`] or [`AccountRefMut::load`],
    /// the account's owner is checked against this ID.
    const PROGRAM_ID: Address;
}

/// Standard errors used by framework wrappers.
///
/// These functions provide consistent error codes for common failure cases
/// in account loading and validation. Using functions instead of constants
/// allows for potential future customization and clearer stack traces.
pub mod errors {
    use pinocchio::error::ProgramError;

    /// Error returned when a mutable operation is attempted on a non-writable account.
    ///
    /// This is returned by [`AccountRefMut::load`] when `info.is_writable()` is false.
    #[inline]
    pub fn account_not_writable() -> ProgramError {
        ProgramError::InvalidAccountData
    }

    /// Error returned when account data is invalid.
    ///
    /// This covers several cases:
    /// - Account data is shorter than the expected type size
    /// - Account discriminator doesn't match the expected type
    /// - Account data fails other structural validation
    #[inline]
    pub fn invalid_account_data() -> ProgramError {
        ProgramError::InvalidAccountData
    }

    /// Error returned when trying to initialize an already-initialized account.
    ///
    /// This is returned by [`AccountRefMut::init`] when the account already
    /// has a non-zero discriminator or is not system-owned.
    #[inline]
    pub fn account_already_initialized() -> ProgramError {
        ProgramError::AccountAlreadyInitialized
    }
}

/// Metadata for a single account in a Shank-compatible instruction.
///
/// This struct is generated by the `#[instruction]` macro (on structs) for each
/// field in the instruction struct. It captures the information needed to
/// generate Shank IDL `#[account]` attributes.
///
/// # Example
///
/// ```ignore
/// // Generated by #[instruction] on a struct:
/// const ACCOUNTS: [ShankAccountMeta; 3] = [
///     ShankAccountMeta { index: 0, name: "source", is_signer: false, is_writable: true, is_program: false },
///     ShankAccountMeta { index: 1, name: "destination", is_signer: false, is_writable: true, is_program: false },
///     ShankAccountMeta { index: 2, name: "owner", is_signer: true, is_writable: false, is_program: false },
/// ];
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ShankAccountMeta {
    /// Account index in the accounts array (0-based).
    pub index: usize,
    /// Account name for IDL generation.
    pub name: &'static str,
    /// Whether this account must be a signer.
    pub is_signer: bool,
    /// Whether this account must be writable.
    pub is_writable: bool,
    /// Whether this account is a program account.
    pub is_program: bool,
}

impl ShankAccountMeta {
    /// Format this account metadata as a Shank `#[account(...)]` attribute.
    pub fn to_shank_attribute(&self) -> alloc::string::String {
        let mut parts = alloc::vec::Vec::new();
        parts.push(alloc::format!("{}", self.index));

        if self.is_writable {
            parts.push(alloc::string::String::from("writable"));
        }
        if self.is_signer {
            parts.push(alloc::string::String::from("signer"));
        }

        parts.push(alloc::format!("name=\"{}\"", self.name));

        alloc::format!("#[account({})]", parts.join(", "))
    }
}

extern crate alloc;

/// Define framework type aliases for account wrappers.
///
/// This macro creates program-specific type aliases that bake in your program ID,
/// so you don't need to pass it around or specify it on every account load.
///
/// # What It Creates
///
/// - `Solzempic`: A struct implementing [`Framework`] with your program ID
/// - `AccountRef<'a, T>`: Type alias for [`AccountRef`]`<'a, T, Solzempic>`
/// - `AccountRefMut<'a, T>`: Type alias for [`AccountRefMut`]`<'a, T, Solzempic>`
/// - `ShardRefContext<'a, T>`: Type alias for [`ShardRefContext`]`<'a, T, Solzempic>`
///
/// # Example
///
/// ```ignore
/// // In your lib.rs:
/// pub const ID: Pubkey = pinocchio_pubkey::pubkey!("Your1111111111111111111111111111111111");
/// solzempic::define_framework!(ID);
///
/// // Now in your instructions:
/// pub struct MyInstruction<'a> {
///     pub account: AccountRefMut<'a, MyAccount>,  // No need to specify Solzempic
/// }
///
/// // Loading automatically checks ownership:
/// let account = AccountRefMut::load(&accounts[0])?;  // Checks owner == ID
/// ```
///
/// # Placement
///
/// Call this macro at the crate root (in `lib.rs`) so the type aliases are
/// available throughout your program. The generated types are public, so they
/// can be used in any module.
#[macro_export]
macro_rules! define_framework {
    ($program_id:expr) => {
        /// Program-specific framework implementation.
        ///
        /// This struct is generated by [`define_framework!`] and implements
        /// [`Framework`] with your program's ID.
        pub struct Solzempic;

        impl $crate::Framework for Solzempic {
            const PROGRAM_ID: solana_address::Address = $program_id;
        }

        /// Read-only account wrapper for Pod types.
        ///
        /// This is a type alias for [`solzempic::AccountRef`] with your program's
        /// framework baked in. Accounts loaded with this type have their owner
        /// validated against your program ID.
        pub type AccountRef<'a, T> = $crate::AccountRef<'a, T, Solzempic>;

        /// Writable account wrapper for Pod types.
        ///
        /// This is a type alias for [`solzempic::AccountRefMut`] with your program's
        /// framework baked in. Accounts loaded with this type have their owner
        /// and `is_writable` flag validated.
        pub type AccountRefMut<'a, T> = $crate::AccountRefMut<'a, T, Solzempic>;

        /// Context holding AccountRefMuts for prev, current, and next shards.
        ///
        /// This is a type alias for [`solzempic::ShardRefContext`] with your program's
        /// framework baked in. Use this for sharded data structures that need access
        /// to neighboring shards for rebalancing or traversal.
        pub type ShardRefContext<'a, T> = $crate::ShardRefContext<'a, T, Solzempic>;

        /// Returns the program ID.
        #[inline]
        pub fn id() -> &'static solana_address::Address {
            &$program_id
        }
    };
}
