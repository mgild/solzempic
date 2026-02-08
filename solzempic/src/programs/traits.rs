//! Trait for validated program and sysvar account wrappers.
//!
//! This module defines the [`ValidatedAccount`] trait, which provides a
//! consistent interface for all program and sysvar account wrappers.
//!
//! Also defines [`HasAccountView`] for ergonomic function parameters that
//! accept either raw `&AccountView` or validated wrappers.

use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;

/// Trait for types that can provide an address.
///
/// This trait enables ergonomic function signatures that accept
/// `AccountView`, `AccountRef`, `AccountRefMut`, or any validated wrapper.
///
/// # Example
///
/// ```ignore
/// fn check_owner(account: &impl HasAddress) -> bool {
///     account.address() == &expected_address
/// }
///
/// // All work:
/// check_owner(&account_view);
/// check_owner(&account_ref);
/// check_owner(&signer);
/// ```
pub trait HasAddress {
    fn address(&self) -> &Address;
}

/// Trait for types that can provide a reference to an AccountView.
///
/// This trait enables ergonomic function signatures that accept both
/// raw `&AccountView` and validated wrapper types like `Signer`, `Mint`, etc.
///
/// # Example
///
/// ```ignore
/// fn do_something(account: impl HasAccountView) {
///     let view = account.account_view();
///     // use view...
/// }
///
/// // Both work:
/// do_something(&raw_account_view);
/// do_something(&signer);
/// ```
pub trait HasAccountView {
    fn account_view(&self) -> &AccountView;
}

impl HasAddress for AccountView {
    #[inline]
    fn address(&self) -> &Address {
        self.address()
    }
}


impl HasAccountView for AccountView {
    #[inline]
    fn account_view(&self) -> &AccountView {
        self
    }
}

impl<T: HasAccountView> HasAccountView for &T {
    #[inline]
    fn account_view(&self) -> &AccountView {
        (*self).account_view()
    }
}

/// Extension trait for AccountView slices to enable ergonomic validation.
///
/// Provides the `validated()` method for converting accounts without explicit types or references.
///
/// # Example
///
/// ```ignore
/// use solzempic::AccountSliceExt;
///
/// let system_program = accounts.validated(0)?;  // Type inferred from usage
/// let payer = accounts.validated(1)?;
/// ```
pub trait AccountSliceExt {
    /// Get a validated account at the given index.
    ///
    /// # Errors
    ///
    /// Returns `ProgramError::NotEnoughAccountKeys` if index is out of bounds,
    /// or the validation error from the specific account type.
    fn validated<'a, T>(&'a self, index: usize) -> Result<T, ProgramError>
    where
        T: ValidatedAccount<'a> + TryFrom<&'a AccountView, Error = ProgramError>;
}

impl AccountSliceExt for [AccountView] {
    #[inline]
    fn validated<'a, T>(&'a self, index: usize) -> Result<T, ProgramError>
    where
        T: ValidatedAccount<'a> + TryFrom<&'a AccountView, Error = ProgramError>,
    {
        self.get(index)
            .ok_or(ProgramError::NotEnoughAccountKeys)?
            .try_into()
    }
}


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

    /// Get the account's public key.
    ///
    /// Convenience method equivalent to `self.info().address()`.
    #[inline]
    fn pubkey(&self) -> &'a Address {
        self.info().address()
    }
}
