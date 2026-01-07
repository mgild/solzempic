//! Trait for validated program and sysvar account wrappers.
//!
//! This module defines the [`ValidatedAccount`] trait, which provides a
//! consistent interface for all program and sysvar account wrappers.

use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;

/// Trait for validated program and sysvar account wrappers.
///
/// `ValidatedAccount` provides a consistent interface for wrapping accounts
/// that need identity validation (programs, sysvars, signers). Unlike
/// [`AccountRef`](crate::AccountRef) which validates program-owned data accounts,
/// `ValidatedAccount` is for accounts where we validate what the account *is*
/// rather than what data it contains.
///
/// # Implementing Types
///
/// This trait is implemented by:
///
/// | Type | Validates |
/// |------|-----------|
/// | [`SystemProgram`](super::SystemProgram) | Account key == System Program ID |
/// | [`TokenProgram`](super::TokenProgram) | Account key == Token or Token-2022 ID |
/// | [`AtaProgram`](super::AtaProgram) | Account key == ATA Program ID |
/// | [`AltProgram`](super::AltProgram) | Account key == ALT Program ID |
/// | [`Signer`](super::Signer) | Account `is_signer == true` |
/// | [`ClockSysvar`](super::ClockSysvar) | Account key == Clock sysvar ID |
/// | [`RentSysvar`](super::RentSysvar) | Account key == Rent sysvar ID |
/// | [`SlotHashesSysvar`](super::SlotHashesSysvar) | Account key == SlotHashes ID |
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, SystemProgram, TokenProgram, Signer};
///
/// fn validate_accounts<'a>(accounts: &'a [AccountInfo]) -> Result<(), ProgramError> {
///     let signer = Signer::wrap(&accounts[0])?;
///     let system_program = SystemProgram::wrap(&accounts[1])?;
///     let token_program = TokenProgram::wrap(&accounts[2])?;
///
///     // All accounts are now validated
///     Ok(())
/// }
/// ```
///
/// # Generic Usage
///
/// The trait allows writing generic code over validated accounts:
///
/// ```ignore
/// fn log_account_key<'a, T: ValidatedAccount<'a>>(account: &T) {
///     msg!("Account key: {:?}", account.key());
/// }
/// ```
///
/// # Comparison with AccountRef
///
/// | Aspect | `ValidatedAccount` | `AccountRef` |
/// |--------|-------------------|--------------|
/// | Purpose | Program/sysvar identity | Program-owned data |
/// | Validates | Key or is_signer flag | Owner + discriminator + size |
/// | Data access | Raw AccountInfo only | Typed `get()` method |
/// | Use case | External programs | Your program's accounts |
pub trait ValidatedAccount<'a>: Sized {
    /// Validate and wrap an AccountView.
    ///
    /// This method checks that the account meets the type's requirements
    /// (correct program ID, correct sysvar address, or has signed, depending
    /// on the implementing type).
    ///
    /// # Arguments
    ///
    /// * `info` - The AccountView to validate and wrap
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails:
    /// - [`ProgramError::IncorrectProgramId`] - For program/sysvar wrappers
    /// - [`ProgramError::MissingRequiredSignature`] - For `Signer` wrapper
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError>;

    /// Get a reference to the underlying AccountView.
    ///
    /// This provides access to all AccountView fields for advanced use cases.
    fn info(&self) -> &'a AccountView;

    /// Get the account's address.
    ///
    /// Convenience method equivalent to `self.info().address()`.
    #[inline]
    fn address(&self) -> &'a Address {
        self.info().address()
    }
}
