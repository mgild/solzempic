//! Address Lookup Table Program wrapper.
//!
//! This module provides [`AltProgram`], a validated wrapper for the
//! Address Lookup Table (ALT) program.

use pinocchio::{AccountView, error::ProgramError};
use solana_address::address_eq;

use super::ids::ADDRESS_LOOKUP_TABLE_PROGRAM_ID;
use super::traits::ValidatedAccount;

/// Validated Address Lookup Table Program wrapper.
///
/// The ALT program enables transaction compression by storing frequently-used
/// addresses in lookup tables. Transactions can then reference addresses by
/// 1-byte index instead of 32-byte pubkey.
///
/// # Transaction Size Savings
///
/// | Without ALT | With ALT |
/// |-------------|----------|
/// | 32 bytes/address | 1 byte/address |
///
/// For transactions with 10+ accounts, this can save hundreds of bytes.
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, AltProgram};
///
/// fn setup_lookup_table<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let alt_program = AltProgram::wrap(&accounts[0])?;
///
///     // Use alt_program.info() in CPI to create/extend lookup tables
///     Ok(())
/// }
/// ```
///
/// # When to Use
///
/// Include `AltProgram` when:
/// - Creating new lookup tables
/// - Extending existing lookup tables with new addresses
/// - Closing/deactivating lookup tables
///
/// # Performance
///
/// Validation cost: ~20 CUs (single 32-byte key comparison)
///
/// # See Also
///
/// - [`Lut`](super::Lut) - Wrapper for lookup table accounts themselves
pub struct AltProgram<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for AltProgram<'a> {
    /// Validate that the account is the ALT program.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::IncorrectProgramId`] if the account key
    /// does not match [`ADDRESS_LOOKUP_TABLE_PROGRAM_ID`].
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !address_eq(info.address(), &ADDRESS_LOOKUP_TABLE_PROGRAM_ID) {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}
