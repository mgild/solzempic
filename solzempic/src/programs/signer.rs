//! Signer and Payer account wrappers.
//!
//! This module provides [`Signer`] and [`Payer`] types for validated
//! signer accounts. These ensure that an account has actually signed
//! the transaction.

use pinocchio::{AccountView, error::ProgramError};

use super::traits::ValidatedAccount;

/// Validated signer account wrapper.
///
/// `Signer` wraps an AccountInfo that has been validated to have signed
/// the transaction (`is_signer == true`). This provides type-level
/// assurance that signature verification has been performed.
///
/// # Why Use Signer?
///
/// Without `Signer`, it's easy to forget to check `is_signer()`:
///
/// ```ignore
/// // DANGEROUS: No signature check!
/// fn transfer(accounts: &[AccountInfo]) -> ProgramResult {
///     let from = &accounts[0];  // Anyone could pass any account!
///     // ... transfer from account
/// }
///
/// // SAFE: Signer wrapper validates at load time
/// fn transfer(accounts: &[AccountInfo]) -> ProgramResult {
///     let from = Signer::wrap(&accounts[0])?;  // Fails if not signer
///     // ... transfer from account
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, Signer};
///
/// pub struct Deposit<'a> {
///     pub user: Signer<'a>,
///     pub vault: AccountRefMut<'a, Vault>,
/// }
///
/// impl<'a> Deposit<'a> {
///     pub fn build(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
///         Ok(Self {
///             user: Signer::wrap(&accounts[0])?,  // Must have signed
///             vault: AccountRefMut::load(&accounts[1])?,
///         })
///     }
/// }
/// ```
///
/// # What Makes an Account a Signer?
///
/// An account is a signer if:
/// - Its keypair was used to sign the transaction, OR
/// - It's a PDA signing via `invoke_signed`
///
/// The `is_signer` flag is set by the Solana runtime and cannot be
/// spoofed by instruction data.
///
/// # Performance
///
/// Validation cost: ~5 CUs (single boolean check)
///
/// # See Also
///
/// - [`Payer`] - Type alias for signers that pay for transactions
pub struct Signer<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for Signer<'a> {
    /// Validate that the account has signed the transaction.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::MissingRequiredSignature`] if
    /// `info.is_signer()` is false.
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}

/// Type alias for payer accounts.
///
/// `Payer` is semantically identical to [`Signer`], but signals that
/// this account is expected to pay for transaction fees or rent.
///
/// # When to Use Payer vs Signer
///
/// - Use `Payer` when the account pays rent for new accounts
/// - Use `Signer` for general authorization (ownership, permission)
///
/// ```ignore
/// pub struct CreateMarket<'a> {
///     pub payer: Payer<'a>,    // Pays rent for new market account
///     pub admin: Signer<'a>,   // Authorized to create markets
///     pub market: AccountRefMut<'a, Market>,
/// }
/// ```
///
/// # Note
///
/// This is purely a semantic distinction. Both types perform the same
/// `is_signer()` validation.
pub type Payer<'a> = Signer<'a>;
