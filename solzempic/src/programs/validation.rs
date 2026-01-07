//! Convenience validation functions for program and sysvar accounts.
//!
//! This module provides lightweight validation functions that verify an account
//! matches an expected program or sysvar ID. These are simpler alternatives to
//! the wrapper types when you only need validation, not access to additional methods.
//!
//! # When to Use
//!
//! | Need | Use |
//! |------|-----|
//! | Just validate, no methods | Validation functions (this module) |
//! | Validation + methods | Wrapper types ([`SystemProgram`], [`ClockSysvar`], etc.) |
//! | Token program (either) | [`validate_token_program`] |
//!
//! # Performance
//!
//! Each function costs ~20 CUs (single 32-byte comparison). Wrapper types have
//! identical validation cost but provide additional functionality.
//!
//! # Example
//!
//! ```ignore
//! use solzempic::validation::{validate_system_program, validate_token_program};
//!
//! fn my_instruction(accounts: &[AccountInfo]) -> ProgramResult {
//!     // Quick validation without wrapping
//!     validate_system_program(&accounts[0])?;
//!     validate_token_program(&accounts[1])?;
//!
//!     // Now we know accounts[0] is System, accounts[1] is Token/Token-2022
//!     Ok(())
//! }
//! ```
//!
//! # See Also
//!
//! - [`ValidatedAccount`](super::ValidatedAccount) - Trait for wrapper-based validation
//! - [`SystemProgram`](super::SystemProgram), [`TokenProgram`](super::TokenProgram) - Wrapper types

use pinocchio::{error::ProgramError, AccountView};
use solana_address::address_eq;

use super::ids::*;

/// Internal macro to define validation functions for program/sysvar IDs.
///
/// Generates inline validation functions that compare an account's key
/// against a known program or sysvar ID.
macro_rules! define_validator {
    ($fn_name:ident, $id:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// # Errors
        ///
        /// Returns [`ProgramError::IncorrectProgramId`] if the account key
        /// does not match the expected ID.
        ///
        /// # Performance
        ///
        /// ~20 CUs (4 u64 comparisons via address_eq)
        #[inline]
        pub fn $fn_name(account: &AccountView) -> Result<(), ProgramError> {
            if !address_eq(account.address(), &$id) {
                return Err(ProgramError::IncorrectProgramId);
            }
            Ok(())
        }
    };
}

/// Validate that an account is a token program (SPL Token or Token-2022).
///
/// This function accepts either the original SPL Token program or the newer
/// Token-2022 program, making it ideal for instructions that should work
/// with both token standards.
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] if the account key is neither
/// [`TOKEN_PROGRAM_ID`] nor [`TOKEN_2022_PROGRAM_ID`].
///
/// # Example
///
/// ```ignore
/// use solzempic::validation::validate_token_program;
///
/// fn transfer(accounts: &[AccountInfo]) -> ProgramResult {
///     let token_program = &accounts[4];
///     validate_token_program(token_program)?;
///
///     // Safe to invoke - we know it's a valid token program
///     invoke(&transfer_ix, &[...])?;
///     Ok(())
/// }
/// ```
///
/// # Performance
///
/// ~40 CUs worst case (two address_eq comparisons if first fails)
///
/// # See Also
///
/// - [`TokenProgram`](super::TokenProgram) - Wrapper type with additional methods
#[inline]
pub fn validate_token_program(account: &AccountView) -> Result<(), ProgramError> {
    let key = account.address();
    if !address_eq(key, &TOKEN_PROGRAM_ID) && !address_eq(key, &TOKEN_2022_PROGRAM_ID) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

define_validator!(validate_system_program, SYSTEM_PROGRAM_ID, "Validate that an account is the System Program.");
define_validator!(validate_clock_sysvar, CLOCK_SYSVAR_ID, "Validate that an account is the Clock sysvar.");
define_validator!(validate_slot_hashes_sysvar, SLOT_HASHES_SYSVAR_ID, "Validate that an account is the SlotHashes sysvar.");
define_validator!(validate_rent_sysvar, RENT_SYSVAR_ID, "Validate that an account is the Rent sysvar.");
