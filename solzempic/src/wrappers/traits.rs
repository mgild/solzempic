//! Traits for account wrappers.
//!
//! This module defines the [`AsAccountRef`] trait which provides a common
//! interface for both read-only ([`AccountRef`](super::AccountRef)) and
//! writable ([`AccountRefMut`](super::AccountRefMut)) account wrappers.

use pinocchio::AccountView;
use solana_address::Address;

use crate::{Framework, Loadable};

/// Common interface for account wrappers (both read-only and writable).
///
/// This trait is implemented by both [`AccountRef`](super::AccountRef) and
/// [`AccountRefMut`](super::AccountRefMut), allowing generic code to work
/// with either wrapper type when only read access is needed.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the borrowed AccountInfo
/// * `T` - The account data type (must implement [`Loadable`])
/// * `F` - The framework type (must implement [`Framework`](crate::Framework))
///
/// # Example
///
/// ```ignore
/// use solzempic::{AsAccountRef, Framework, Loadable};
///
/// // Generic function that works with any account wrapper
/// fn check_owner<'a, T, F, A>(account: &A, expected: &Pubkey) -> bool
/// where
///     T: Loadable,
///     F: Framework,
///     A: AsAccountRef<'a, T, F>,
/// {
///     account.key() == expected
/// }
///
/// // Works with both AccountRef and AccountRefMut
/// let read_only: AccountRef<MyAccount> = AccountRef::load(&accounts[0])?;
/// let writable: AccountRefMut<MyAccount> = AccountRefMut::load(&accounts[1])?;
///
/// check_owner(&read_only, &expected_key);
/// check_owner(&writable, &expected_key);
/// ```
///
/// # When to Use
///
/// Use this trait when writing generic functions that:
/// - Need to read account data but not modify it
/// - Should work with both read-only and writable wrappers
/// - Need to validate PDA derivation
///
/// If you need mutable access, require `AccountRefMut` directly instead.
pub trait AsAccountRef<'a, T: Loadable, F: Framework> {
    /// Get the underlying AccountView.
    ///
    /// This provides access to all raw AccountView fields like `lamports()`,
    /// `owner()`, `is_signer()`, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let lamports = account.info().lamports();
    /// let is_signer = account.info().is_signer();
    /// ```
    fn info(&self) -> &'a AccountView;

    /// Get the account's address.
    ///
    /// This is a convenience method equivalent to `info().address()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if account.address() == expected_address {
    ///     // Account is at expected address
    /// }
    /// ```
    fn address(&self) -> &Address;

    /// Get a reference to the parsed account data.
    ///
    /// Returns a typed reference to the account's data, parsed as type `T`.
    /// The data is accessed via zero-copy pointer cast from the raw bytes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let counter = account.get();
    /// log::info!("Count: {}", counter.count);
    /// ```
    ///
    /// # Panics
    ///
    /// This method should not panic if the account was successfully loaded,
    /// as size validation happens during [`AccountRef::load`](super::AccountRef::load).
    fn get(&self) -> &T;

    /// Check if this account is a PDA derived from the given seeds.
    ///
    /// Derives the expected PDA from the seeds and the framework's program ID,
    /// then compares it to this account's address.
    ///
    /// # Arguments
    ///
    /// * `seeds` - The PDA seeds (without the bump)
    ///
    /// # Returns
    ///
    /// A tuple of `(is_valid, bump)`:
    /// - `is_valid` - `true` if the account address matches the derived PDA
    /// - `bump` - The canonical bump seed for this PDA
    ///
    /// # Example
    ///
    /// ```ignore
    /// let seeds = &[b"user", owner.key().as_ref()];
    /// let (is_valid, bump) = account.is_pda(seeds);
    ///
    /// if !is_valid {
    ///     return Err(ProgramError::InvalidSeeds);
    /// }
    ///
    /// // Store bump for later use
    /// let full_seeds = &[b"user", owner.key().as_ref(), &[bump]];
    /// ```
    ///
    /// # Performance
    ///
    /// This method derives a PDA which is computationally expensive (~2000 CUs).
    /// If you already know the bump, prefer storing it and validating with a
    /// simple key comparison instead.
    fn is_pda(&self, seeds: &[&[u8]]) -> (bool, u8);
}
