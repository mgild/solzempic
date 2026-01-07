//! Read-only account wrapper.
//!
//! This module provides [`AccountRef`], a zero-overhead wrapper for read-only
//! access to program-owned accounts.

use core::marker::PhantomData;

use pinocchio::{error::ProgramError, AccountView};
use solana_address::{Address, address_eq};

use crate::{check_discriminator, Framework, Loadable};

use super::traits::AsAccountRef;

/// Read-only account wrapper for typed account data.
///
/// `AccountRef` provides safe, zero-copy read access to account data. It validates
/// ownership and data structure on load, then provides typed access to the parsed data.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the borrowed AccountInfo
/// * `T` - The account data type (must implement [`Loadable`])
/// * `F` - The framework type (must implement [`Framework`](crate::Framework))
///
/// # Validation on Load
///
/// When calling [`load`](Self::load), the following checks are performed:
///
/// 1. **Ownership**: Account must be owned by `F::PROGRAM_ID`
/// 2. **Size**: Account data must be at least `T::LEN` bytes
/// 3. **Discriminator**: First bytes must match `T::DISCRIMINATOR`
///
/// # Example
///
/// ```ignore
/// use solzempic::AccountRef;
///
/// // Load a counter account (validates owner, size, discriminator)
/// let counter: AccountRef<Counter> = AccountRef::load(&accounts[0])?;
///
/// // Read the data
/// let count = counter.get().count;
/// ```
///
/// # When to Use
///
/// Use `AccountRef` when you need to:
/// - Read account data without modifying it
/// - Validate that an account has the expected structure
/// - Pass account data to validation functions
///
/// If you need to modify the account, use [`AccountRefMut`](super::AccountRefMut) instead.
///
/// # Performance
///
/// | Operation | Cost |
/// |-----------|------|
/// | `load()` | ~50 CUs (ownership check + discriminator check) |
/// | `get()` | ~5 CUs (pointer cast) |
/// | `is_pda()` | ~2000 CUs (PDA derivation) |
///
/// # See Also
///
/// - [`AccountRefMut`](super::AccountRefMut) - Writable version
/// - [`AsAccountRef`] - Common trait for both wrappers
/// - [`Framework`](crate::Framework) - Program ID configuration
pub struct AccountRef<'a, T: Loadable, F: Framework> {
    /// The underlying AccountView reference.
    pub info: &'a AccountView,
    data: &'a [u8],
    _marker: PhantomData<(T, F)>,
}

impl<'a, T: Loadable, F: Framework> AccountRef<'a, T, F> {
    /// Load and validate an already-initialized account.
    ///
    /// This is the primary way to create an `AccountRef`. It performs full
    /// validation of the account's ownership and data structure.
    ///
    /// # Validation
    ///
    /// 1. Account owner must equal `F::PROGRAM_ID`
    /// 2. Account data must be at least `T::LEN` bytes
    /// 3. Account discriminator must match `T::DISCRIMINATOR`
    ///
    /// # Arguments
    ///
    /// * `info` - The AccountInfo to wrap
    ///
    /// # Errors
    ///
    /// * [`ProgramError::IllegalOwner`] - Account not owned by this program
    /// * [`ProgramError::InvalidAccountData`] - Data too small or wrong discriminator
    ///
    /// # Example
    ///
    /// ```ignore
    /// let market: AccountRef<Market> = AccountRef::load(&accounts[0])?;
    /// ```
    #[inline]
    pub fn load(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !address_eq(unsafe { info.owner() }, &F::PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }
        Self::load_unchecked(info)
    }

    /// Load an account without ownership validation.
    ///
    /// This skips the ownership check but still validates data size and
    /// discriminator. Use this when you know the account may be owned by
    /// a different program (e.g., cross-program reads).
    ///
    /// # Warning
    ///
    /// Only use this if you have a specific reason to skip ownership validation.
    /// For normal program accounts, use [`load`](Self::load) instead.
    ///
    /// # Errors
    ///
    /// * [`ProgramError::InvalidAccountData`] - Data too small or wrong discriminator
    #[inline]
    pub fn load_unchecked(info: &'a AccountView) -> Result<Self, ProgramError> {
        let data = unsafe { info.borrow_unchecked() };

        if data.len() < T::LEN {
            return Err(crate::errors::invalid_account_data());
        }

        if !check_discriminator(data, T::DISCRIMINATOR) {
            return Err(crate::errors::invalid_account_data());
        }

        Ok(Self {
            info,
            data,
            _marker: PhantomData,
        })
    }

    /// Get the account's address.
    ///
    /// Convenience method equivalent to `self.info.address()`.
    #[inline]
    pub fn address(&self) -> &Address {
        self.info.address()
    }

    /// Get a reference to the parsed account data.
    ///
    /// Returns a typed reference to the account's data via zero-copy pointer cast.
    /// This is extremely cheap (~5 CUs) and can be called repeatedly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let market = market_ref.get();
    /// let base_mint = &market.base_mint;
    /// let quote_mint = &market.quote_mint;
    /// ```
    #[inline]
    pub fn get(&self) -> &T {
        bytemuck::from_bytes(&self.data[..T::LEN])
    }

    /// Check if this account is a PDA derived from the given seeds.
    ///
    /// Derives the expected PDA address from the seeds and framework's program ID,
    /// then compares it against this account's address.
    ///
    /// # Arguments
    ///
    /// * `seeds` - The PDA seeds (without the bump)
    ///
    /// # Returns
    ///
    /// A tuple of `(is_valid, bump)`:
    /// - `is_valid` - `true` if the account address matches the derived PDA
    /// - `bump` - The canonical bump seed for this derivation
    ///
    /// # Example
    ///
    /// ```ignore
    /// let seeds = &[b"market", base_mint.as_ref(), quote_mint.as_ref()];
    /// let (is_valid, bump) = market.is_pda(seeds);
    ///
    /// if !is_valid {
    ///     return Err(ProgramError::InvalidSeeds);
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// PDA derivation is expensive (~2000 CUs). For frequent validation, consider
    /// storing the bump and using a direct key comparison instead.
    #[inline]
    pub fn is_pda(&self, seeds: &[&[u8]]) -> (bool, u8) {
        let (expected, bump) = Address::find_program_address(seeds, &F::PROGRAM_ID);
        (self.info.address().as_ref() == expected.as_ref(), bump)
    }
}

impl<'a, T: Loadable, F: Framework> AsAccountRef<'a, T, F> for AccountRef<'a, T, F> {
    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }

    #[inline]
    fn address(&self) -> &Address {
        self.info.address()
    }

    #[inline]
    fn get(&self) -> &T {
        bytemuck::from_bytes(&self.data[..T::LEN])
    }

    #[inline]
    fn is_pda(&self, seeds: &[&[u8]]) -> (bool, u8) {
        let (expected, bump) = Address::find_program_address(seeds, &F::PROGRAM_ID);
        (self.info.address().as_ref() == expected.as_ref(), bump)
    }
}
