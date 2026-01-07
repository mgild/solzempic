//! System Program account wrapper.
//!
//! This module provides [`SystemProgram`], a validated wrapper for the
//! Solana System Program account.

use pinocchio::{AccountView, error::ProgramError};
use solana_address::address_eq;

use super::ids::SYSTEM_PROGRAM_ID;
use super::traits::ValidatedAccount;

/// Validated System Program account wrapper.
///
/// `SystemProgram` wraps an AccountInfo that has been validated to be the
/// Solana System Program. Use this to ensure type-safe handling of System
/// program accounts in your instruction handlers.
///
/// # What is the System Program?
///
/// The System Program is Solana's native program for:
/// - Creating new accounts
/// - Allocating data space
/// - Assigning account ownership
/// - Transferring lamports between system-owned accounts
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, SystemProgram};
///
/// fn create_account<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let payer = &accounts[0];
///     let new_account = &accounts[1];
///     let system_program = SystemProgram::wrap(&accounts[2])?;
///
///     // Now safe to use system_program.info() in CPI calls
///     create_pda_account(payer, new_account, &program_id, 256, seeds)?;
///     Ok(())
/// }
/// ```
///
/// # When to Use
///
/// Include `SystemProgram` in your instruction context when:
/// - Creating new accounts via CPI
/// - Transferring lamports via CPI
/// - Allocating or assigning accounts
///
/// # Performance
///
/// Validation cost: ~20 CUs (single 32-byte key comparison)
pub struct SystemProgram<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for SystemProgram<'a> {
    /// Validate that the account is the System Program.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::IncorrectProgramId`] if the account key
    /// does not match [`SYSTEM_PROGRAM_ID`].
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !address_eq(info.address(), &SYSTEM_PROGRAM_ID) {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}
