//! SPL Token Program account wrapper.
//!
//! This module provides [`TokenProgram`], a validated wrapper for either
//! the SPL Token program or Token-2022 program.

use pinocchio::{error::ProgramError, AccountView};
use solana_address::address_eq;

use super::ids::{TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};
use super::traits::ValidatedAccount;

/// Validated SPL Token Program account wrapper.
///
/// `TokenProgram` wraps an AccountInfo that has been validated to be either
/// the SPL Token program or the Token-2022 (Token Extensions) program.
/// This flexibility allows your program to work with both token standards.
///
/// # Token vs Token-2022
///
/// | Feature | SPL Token | Token-2022 |
/// |---------|-----------|------------|
/// | Transfer hooks | No | Yes |
/// | Confidential transfers | No | Yes |
/// | Transfer fees | No | Yes |
/// | Interest-bearing | No | Yes |
/// | Non-transferable | No | Yes |
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, TokenProgram};
///
/// fn transfer_tokens<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let token_program = TokenProgram::wrap(&accounts[3])?;
///
///     // Check which token program we're using
///     if token_program.is_token_2022() {
///         msg!("Using Token-2022 program");
///     }
///
///     // Use token_program.info() in CPI calls
///     invoke(
///         &transfer_instruction,
///         &[source, destination, authority, token_program.info()],
///     )?;
///     Ok(())
/// }
/// ```
///
/// # When to Use
///
/// Include `TokenProgram` in your instruction context when:
/// - Transferring tokens via CPI
/// - Minting or burning tokens
/// - Approving delegates
/// - Any token operation via CPI
///
/// # Performance
///
/// Validation cost: ~40 CUs (two 32-byte key comparisons)
pub struct TokenProgram<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for TokenProgram<'a> {
    /// Validate that the account is a token program (Token or Token-2022).
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::IncorrectProgramId`] if the account key
    /// matches neither [`TOKEN_PROGRAM_ID`] nor [`TOKEN_2022_PROGRAM_ID`].
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        let key = info.address();
        if !address_eq(key, &TOKEN_PROGRAM_ID) && !address_eq(key, &TOKEN_2022_PROGRAM_ID) {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}

impl<'a> TokenProgram<'a> {
    /// Check if this is the Token-2022 program.
    ///
    /// Returns `true` if this wrapper holds the Token-2022 program,
    /// `false` if it's the original SPL Token program.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if token_program.is_token_2022() {
    ///     // Handle Token-2022 specific logic (transfer fees, hooks, etc.)
    /// }
    /// ```
    #[inline]
    pub fn is_token_2022(&self) -> bool {
        address_eq(self.info.address(), &TOKEN_2022_PROGRAM_ID)
    }
}
