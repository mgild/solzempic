//! Associated Token Account Program wrapper.
//!
//! This module provides [`AtaProgram`], a validated wrapper for the
//! Associated Token Account (ATA) program.

use pinocchio::{AccountView, error::ProgramError};
use solana_address::address_eq;

use super::ids::ASSOCIATED_TOKEN_PROGRAM_ID;
use super::traits::ValidatedAccount;

/// Validated Associated Token Account Program wrapper.
///
/// The ATA program creates and manages deterministic token account addresses.
/// Given a wallet and mint, the ATA address is uniquely derived:
///
/// ```text
/// ATA = PDA([wallet, TOKEN_PROGRAM_ID, mint], ATA_PROGRAM_ID)
/// ```
///
/// # Why Use ATAs?
///
/// - **Deterministic**: Anyone can compute a wallet's token address
/// - **No coordination**: Senders don't need wallet owner to create accounts first
/// - **Idempotent creation**: Safe to call create multiple times
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, AtaProgram, TokenAccountRefMut};
///
/// fn create_user_token_account<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let ata_program = AtaProgram::wrap(&accounts[5])?;
///
///     // Create ATA idempotently
///     TokenAccountRefMut::init_ata(
///         &user_ata,
///         payer.info(),
///         user.info(),
///         &mint,
///         system_program.info(),
///         token_program.info(),
///         ata_program.info(),
///     )?;
///
///     Ok(())
/// }
/// ```
///
/// # Performance
///
/// Validation cost: ~20 CUs (single 32-byte key comparison)
pub struct AtaProgram<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for AtaProgram<'a> {
    /// Validate that the account is the ATA program.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::IncorrectProgramId`] if the account key
    /// does not match [`ASSOCIATED_TOKEN_PROGRAM_ID`].
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !address_eq(info.address(), &ASSOCIATED_TOKEN_PROGRAM_ID) {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}
