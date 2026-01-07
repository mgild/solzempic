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

impl<'a> Signer<'a> {
    /// Returns the account's public key.
    #[inline]
    pub fn address(&self) -> &solana_address::Address {
        self.info.address()
    }

    /// Alias for address() - returns the account's public key.
    #[inline]
    pub fn key(&self) -> &solana_address::Address {
        self.info.address()
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

/// Validated signer account wrapper that is also writable.
///
/// `MutSigner` wraps an AccountInfo that has been validated to have signed
/// the transaction AND be writable. Use this when the signer's account data
/// or lamports will be modified (e.g., the payer paying for rent).
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, MutSigner};
///
/// pub struct CloseAccount<'a> {
///     pub owner: MutSigner<'a>,  // Signer + receives lamports back
///     pub account_to_close: AccountRefMut<'a, MyAccount>,
/// }
/// ```
///
/// # Shank IDL
///
/// In Shank IDL generation, `MutSigner` produces `signer, writable` constraints.
pub struct MutSigner<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for MutSigner<'a> {
    /// Validate that the account has signed the transaction and is writable.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::MissingRequiredSignature`] if not a signer.
    /// Returns [`ProgramError::InvalidAccountData`] if not writable.
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !info.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}

impl<'a> MutSigner<'a> {
    /// Returns the account's public key.
    #[inline]
    pub fn address(&self) -> &solana_address::Address {
        self.info.address()
    }

    /// Alias for address() - returns the account's public key.
    #[inline]
    pub fn key(&self) -> &solana_address::Address {
        self.info.address()
    }
}

/// Explicit wrapper for writable raw account references.
///
/// Use `Writable` when you have a raw `&AccountView` that needs to be
/// marked as writable for Shank IDL generation.
///
/// # Example
///
/// ```ignore
/// pub struct MyInstruction<'a> {
///     pub target: Writable<'a>,  // Writable raw account
/// }
/// ```
///
/// # Shank IDL
///
/// In Shank IDL generation, `Writable` produces a `writable` constraint.
pub struct Writable<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for Writable<'a> {
    /// Validate that the account is writable.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidAccountData`] if not writable.
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}

impl<'a> Writable<'a> {
    /// Returns the account's public key.
    #[inline]
    pub fn address(&self) -> &solana_address::Address {
        self.info.address()
    }

    /// Alias for address() - returns the account's public key.
    #[inline]
    pub fn key(&self) -> &solana_address::Address {
        self.info.address()
    }
}

/// Explicit wrapper for readonly raw account references.
///
/// Use `ReadOnly` when you have a raw `&AccountView` that should be
/// explicitly marked as readonly for Shank IDL generation. This is
/// the same as a bare `&AccountView` reference but makes the intent clearer.
///
/// # Example
///
/// ```ignore
/// pub struct MyInstruction<'a> {
///     pub config: ReadOnly<'a>,  // Readonly raw account
/// }
/// ```
///
/// # Shank IDL
///
/// In Shank IDL generation, `ReadOnly` produces no writable constraint (readonly).
pub struct ReadOnly<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for ReadOnly<'a> {
    /// Wraps the account reference (no validation needed for readonly).
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}

impl<'a> ReadOnly<'a> {
    /// Returns the account's public key.
    #[inline]
    pub fn address(&self) -> &solana_address::Address {
        self.info.address()
    }

    /// Alias for address() - returns the account's public key.
    #[inline]
    pub fn key(&self) -> &solana_address::Address {
        self.info.address()
    }
}
