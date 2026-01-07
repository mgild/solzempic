//! Program and sysvar account wrappers.
//!
//! Type-safe wrappers that validate account IDs on construction.
//! Use these instead of raw AccountInfo for known program/sysvar accounts.
//!
//! # Example
//!
//! ```ignore
//! use solzempic::programs::{ValidatedAccount, SystemProgram, TokenProgram, Signer};
//!
//! fn process(accounts: &[AccountInfo]) -> ProgramResult {
//!     let signer = Signer::wrap(&accounts[0])?;
//!     let system_program = SystemProgram::wrap(&accounts[1])?;
//!     let token_program = TokenProgram::wrap(&accounts[2])?;
//!
//!     // Use validated accounts...
//!     Ok(())
//! }
//! ```

pub mod ids;
mod alt;
mod ata;
mod lut;
mod mint;
mod signer;
mod system;
mod sysvars;
mod token_account;
mod token_program;
mod traits;
mod validation;
mod vault;

// Re-export IDs
pub use ids::*;

// Re-export wrappers
pub use alt::AltProgram;
pub use ata::AtaProgram;
pub use lut::Lut;
pub use mint::Mint;
pub use signer::{MutSigner, Payer, ReadOnly, Signer, Writable};
pub use system::SystemProgram;
pub use sysvars::{ClockSysvar, InstructionsSysvar, RecentBlockhashesSysvar, RentSysvar, SlotHashesSysvar};
pub use token_account::{TokenAccountData, TokenAccountRefMut};
pub use token_program::TokenProgram;
pub use traits::ValidatedAccount;
pub use vault::{SolVault, Vault};
pub use validation::{
    validate_clock_sysvar, validate_rent_sysvar, validate_slot_hashes_sysvar,
    validate_system_program, validate_token_program,
};
