//! Mutable shard reference context for triplet navigation.
//!
//! This module provides [`ShardRefMutContext`], a container for managing three
//! related shard accounts (low, current, high) with mutable access.

use core::ptr;

use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;

use crate::{Framework, Loadable};

use super::account_ref_mut::AccountRefMut;

/// Context holding writable references to a triplet of shards.
///
/// `ShardRefMutContext` manages three [`AccountRefMut`] references for sharded
/// data structures where operations may need to modify neighboring shards.
/// This is commonly used for:
///
/// - **Orderbooks**: Orders may need to move between price-range shards
/// - **Linked lists**: Insertions/deletions update low/high pointers
/// - **Rebalancing**: Moving data between under/over-utilized shards
///
/// # IDL Generation
///
/// When used in instruction structs, all three accounts are marked as writable
/// in the generated IDL with names: `{field}_low_shard`, `{field}_current_shard`, `{field}_high_shard`.
///
/// # Deduplication
///
/// When the same account is passed for multiple positions (e.g., low == current),
/// this struct automatically deduplicates to avoid undefined behavior from
/// aliased mutable references. Accessors return references to the same underlying
/// account when positions are duplicates.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the borrowed AccountInfo references
/// * `T` - The shard data type (must implement [`Loadable`])
/// * `F` - The framework type (must implement [`Framework`](crate::Framework))
///
/// # Example
///
/// ```ignore
/// use solzempic::ShardRefMutContext;
///
/// // Load a shard triplet for a rebalancing operation
/// let mut shards: ShardRefMutContext<OrderShard> = ShardRefMutContext::new(
///     &accounts[0],  // low shard
///     &accounts[1],  // current shard
///     &accounts[2],  // high shard
/// )?;
///
/// // Access shards - duplicates are handled automatically
/// let current_data = shards.current_mut();
/// ```
///
/// # Performance
///
/// Loading a `ShardRefMutContext` loads each unique account once (~50 CUs each).
/// Duplicate accounts are detected by pointer comparison and only loaded once.
pub struct ShardRefMutContext<'a, T: Loadable, F: Framework> {
    /// The low shard in the linked structure (always loaded).
    low: AccountRefMut<'a, T, F>,
    /// The current shard - may alias low if same account.
    current_ref: CurrentRef<'a, T, F>,
    /// The high shard - may alias low or current if same account.
    high_ref: HighRef<'a, T, F>,
}

/// Current shard reference - either owned or aliasing low
enum CurrentRef<'a, T: Loadable, F: Framework> {
    Owned(AccountRefMut<'a, T, F>),
    AliasLow, // Same as low
}

/// High shard reference - either owned or aliasing low/current
enum HighRef<'a, T: Loadable, F: Framework> {
    Owned(AccountRefMut<'a, T, F>),
    AliasLow,     // Same as low
    AliasCurrent, // Same as current (but not low)
}

impl<'a, T: Loadable, F: Framework> ShardRefMutContext<'a, T, F> {
    /// Create a new shard context by loading three account infos.
    ///
    /// Automatically deduplicates accounts that have the same address to avoid
    /// undefined behavior from aliased mutable references. When the same account
    /// is passed for multiple positions, it is only loaded once.
    ///
    /// # Arguments
    ///
    /// * `low_info` - The low shard's AccountInfo
    /// * `current_info` - The current (primary) shard's AccountInfo
    /// * `high_info` - The high shard's AccountInfo
    ///
    /// # Errors
    ///
    /// Returns an error if any unique account fails validation (wrong owner,
    /// not writable, wrong discriminator, etc.).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Works with three different accounts
    /// let shards = ShardRefMutContext::<OrderShard>::new(
    ///     &accounts[0],  // low
    ///     &accounts[1],  // current
    ///     &accounts[2],  // high
    /// )?;
    ///
    /// // Also works when passing the same account multiple times
    /// let shards = ShardRefMutContext::<OrderShard>::new(
    ///     &accounts[0],  // low
    ///     &accounts[0],  // current = same as low
    ///     &accounts[0],  // high = same as low
    /// )?;
    /// ```
    #[inline]
    pub fn new(
        low_info: &'a AccountView,
        current_info: &'a AccountView,
        high_info: &'a AccountView,
    ) -> Result<Self, ProgramError> {
        // Always load low
        let low = AccountRefMut::load(low_info)?;

        // Check if current is same as low (compare raw pointers to avoid loading twice)
        let low_current_same = ptr::eq(low_info, current_info);
        let current_ref = if low_current_same {
            CurrentRef::AliasLow
        } else {
            CurrentRef::Owned(AccountRefMut::load(current_info)?)
        };

        // Check if high is same as low or current
        let low_high_same = ptr::eq(low_info, high_info);
        let current_high_same = ptr::eq(current_info, high_info);
        let high_ref = if low_high_same {
            HighRef::AliasLow
        } else if current_high_same {
            HighRef::AliasCurrent
        } else {
            HighRef::Owned(AccountRefMut::load(high_info)?)
        };

        Ok(Self {
            low,
            current_ref,
            high_ref,
        })
    }

    /// Get the low AccountRefMut reference.
    ///
    /// This provides access to the underlying `AccountRefMut` for operations like
    /// `.address()`, `.data_mut()`, `.info`, `.reload()`, etc.
    #[inline]
    pub fn low_ref(&self) -> &AccountRefMut<'a, T, F> {
        &self.low
    }

    /// Get the low AccountRefMut mutably.
    #[inline]
    pub fn low_ref_mut(&mut self) -> &mut AccountRefMut<'a, T, F> {
        &mut self.low
    }

    /// Get the current AccountRefMut reference.
    ///
    /// This provides access to the underlying `AccountRefMut` for operations like
    /// `.address()`, `.data_mut()`, `.info`, `.reload()`, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let current_addr = shards.current_ref().address();
    /// let current_data = shards.current_ref_mut().data_mut();
    /// ```
    #[inline]
    pub fn current_ref(&self) -> &AccountRefMut<'a, T, F> {
        match &self.current_ref {
            CurrentRef::Owned(acct) => acct,
            CurrentRef::AliasLow => &self.low,
        }
    }

    /// Get the current AccountRefMut mutably.
    #[inline]
    pub fn current_ref_mut(&mut self) -> &mut AccountRefMut<'a, T, F> {
        match &mut self.current_ref {
            CurrentRef::Owned(acct) => acct,
            CurrentRef::AliasLow => &mut self.low,
        }
    }

    /// Get the high AccountRefMut reference.
    ///
    /// This provides access to the underlying `AccountRefMut` for operations like
    /// `.address()`, `.data_mut()`, `.info`, `.reload()`, etc.
    #[inline]
    pub fn high_ref(&self) -> &AccountRefMut<'a, T, F> {
        match &self.high_ref {
            HighRef::Owned(acct) => acct,
            HighRef::AliasLow => &self.low,
            HighRef::AliasCurrent => self.current_ref(),
        }
    }

    /// Get the high AccountRefMut mutably.
    #[inline]
    pub fn high_ref_mut(&mut self) -> &mut AccountRefMut<'a, T, F> {
        // Use sequential checks to avoid nested borrows
        if let HighRef::Owned(ref mut acct) = self.high_ref {
            return acct;
        }
        if matches!(self.high_ref, HighRef::AliasLow) {
            return &mut self.low;
        }
        // AliasCurrent case - current might also alias low
        if let CurrentRef::Owned(ref mut acct) = self.current_ref {
            return acct;
        }
        // Current aliases low, so high also aliases low
        &mut self.low
    }

    // Private aliases for internal use
    #[inline]
    fn current_account(&self) -> &AccountRefMut<'a, T, F> {
        self.current_ref()
    }

    #[inline]
    fn current_account_mut(&mut self) -> &mut AccountRefMut<'a, T, F> {
        self.current_ref_mut()
    }

    #[inline]
    fn high_account(&self) -> &AccountRefMut<'a, T, F> {
        self.high_ref()
    }

    #[inline]
    fn high_account_mut(&mut self) -> &mut AccountRefMut<'a, T, F> {
        self.high_ref_mut()
    }

    /// Try to create a shard context, returning `None` if any account is invalid.
    ///
    /// This is useful for optional opposite-side shards that may not exist yet.
    /// Automatically deduplicates accounts that have the same address.
    ///
    /// # Arguments
    ///
    /// * `low_info` - The low shard's AccountInfo
    /// * `current_info` - The current (primary) shard's AccountInfo
    /// * `high_info` - The high shard's AccountInfo
    ///
    /// # Returns
    ///
    /// `Some(Self)` if all unique accounts are valid and initialized, `None` otherwise.
    #[inline]
    pub fn try_new(
        low_info: &'a AccountView,
        current_info: &'a AccountView,
        high_info: &'a AccountView,
    ) -> Option<Self> {
        let low = AccountRefMut::try_load(low_info)?;

        let low_current_same = ptr::eq(low_info, current_info);
        let current_ref = if low_current_same {
            CurrentRef::AliasLow
        } else {
            CurrentRef::Owned(AccountRefMut::try_load(current_info)?)
        };

        let low_high_same = ptr::eq(low_info, high_info);
        let current_high_same = ptr::eq(current_info, high_info);
        let high_ref = if low_high_same {
            HighRef::AliasLow
        } else if current_high_same {
            HighRef::AliasCurrent
        } else {
            HighRef::Owned(AccountRefMut::try_load(high_info)?)
        };

        Some(Self {
            low,
            current_ref,
            high_ref,
        })
    }

    /// Create a context from already-loaded shard wrappers.
    ///
    /// **Warning**: This does not perform deduplication. The caller must ensure
    /// that all three AccountRefMut refer to different accounts. If you need
    /// deduplication, use [`new`](Self::new) instead.
    #[inline]
    pub fn from_loaded(
        low: AccountRefMut<'a, T, F>,
        current: AccountRefMut<'a, T, F>,
        high: AccountRefMut<'a, T, F>,
    ) -> Self {
        Self {
            low,
            current_ref: CurrentRef::Owned(current),
            high_ref: HighRef::Owned(high),
        }
    }

    /// Get the address of the current shard.
    #[inline]
    pub fn current_address(&self) -> &Address {
        self.current_account().address()
    }

    /// Get the address of the low shard.
    #[inline]
    pub fn low_address(&self) -> &Address {
        self.low.address()
    }

    /// Get the address of the high shard.
    #[inline]
    pub fn high_address(&self) -> &Address {
        self.high_account().address()
    }

    /// Get read-only access to the current shard's data.
    #[inline]
    pub fn current(&self) -> &T {
        self.current_account().get()
    }

    /// Get read-only access to the low shard's data.
    #[inline]
    pub fn low(&self) -> &T {
        self.low.get()
    }

    /// Get read-only access to the high shard's data.
    #[inline]
    pub fn high(&self) -> &T {
        self.high_account().get()
    }

    /// Get mutable access to the current shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// shards.current_mut().order_count += 1;
    /// ```
    #[inline]
    pub fn current_mut(&mut self) -> &mut T {
        self.current_account_mut().get_mut()
    }

    /// Get mutable access to the low shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// shards.low_mut().high_shard = *new_shard_key;
    /// ```
    #[inline]
    pub fn low_mut(&mut self) -> &mut T {
        self.low.get_mut()
    }

    /// Get mutable access to the high shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// shards.high_mut().low_shard = *new_shard_key;
    /// ```
    #[inline]
    pub fn high_mut(&mut self) -> &mut T {
        self.high_account_mut().get_mut()
    }

    /// Get mutable access to all three shards simultaneously.
    ///
    /// Returns a tuple of `(low, current, high)` mutable references.
    /// When accounts are deduplicated (same account for multiple positions),
    /// the returned references will point to the same underlying data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (low, current, high) = shards.all_mut();
    ///
    /// // Move an order from current to high shard
    /// let order = current.remove_order(order_idx);
    /// high.insert_order(order);
    ///
    /// // Update linked list pointers
    /// low.high_shard = *current_key;
    /// high.low_shard = *current_key;
    /// ```
    ///
    /// # Note
    ///
    /// When the same account is passed for multiple positions, this returns
    /// the same reference multiple times. This is safe because Rust's aliasing
    /// rules are satisfied at the AccountRefMut level (only one AccountRefMut
    /// exists per unique account).
    #[inline]
    pub fn all_mut(&mut self) -> (&mut T, &mut T, &mut T) {
        // Safety: We need to return three mutable references that may alias.
        // This is safe because:
        // 1. Each unique AccountView only has one AccountRefMut
        // 2. The deduplication in new() ensures no duplicate borrows
        // 3. We use raw pointers to work around Rust's aliasing rules
        //    for the intentional aliasing case
        let low_ptr = self.low.get_mut() as *mut T;
        let current_ptr = self.current_account_mut().get_mut() as *mut T;
        let high_ptr = self.high_account_mut().get_mut() as *mut T;

        unsafe {
            (&mut *low_ptr, &mut *current_ptr, &mut *high_ptr)
        }
    }

    /// Get mutable access to all three shards' raw data slices simultaneously.
    ///
    /// Returns a tuple of `(low_data, current_data, high_data)` mutable byte slices.
    /// When accounts are deduplicated (same account for multiple positions),
    /// the returned slices will point to the same underlying data.
    ///
    /// This is useful for operations that need to work with the raw account data
    /// (e.g., creating ClmmOrders or LimitOrders views).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (low_data, current_data, high_data) = shards.all_data_mut();
    ///
    /// let mut low_orders = ClmmOrders::from_account(low_data).unwrap();
    /// let mut current_orders = ClmmOrders::from_account(current_data).unwrap();
    /// ```
    #[inline]
    pub fn all_data_mut(&mut self) -> (&mut [u8], &mut [u8], &mut [u8]) {
        // Safety: Same as all_mut() - we use raw pointers to allow intentional aliasing
        let low_ptr = self.low.data_mut() as *mut [u8];
        let current_ptr = self.current_account_mut().data_mut() as *mut [u8];
        let high_ptr = self.high_account_mut().data_mut() as *mut [u8];

        unsafe {
            (&mut *low_ptr, &mut *current_ptr, &mut *high_ptr)
        }
    }

    /// Get all three AccountRefMut references simultaneously.
    ///
    /// Returns a tuple of `(low, current, high)` mutable AccountRefMut references.
    /// When accounts are deduplicated, the returned references may point to the same account.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (low, current, high) = shards.all_refs_mut();
    ///
    /// let low_addr = low.address();
    /// let current_data = current.data_mut();
    /// ```
    #[inline]
    pub fn all_refs_mut(&mut self) -> (&mut AccountRefMut<'a, T, F>, &mut AccountRefMut<'a, T, F>, &mut AccountRefMut<'a, T, F>) {
        // Safety: Same as all_mut() - we use raw pointers to allow intentional aliasing
        let low_ptr = &mut self.low as *mut AccountRefMut<'a, T, F>;
        let current_ptr = self.current_account_mut() as *mut AccountRefMut<'a, T, F>;
        let high_ptr = self.high_account_mut() as *mut AccountRefMut<'a, T, F>;

        unsafe {
            (&mut *low_ptr, &mut *current_ptr, &mut *high_ptr)
        }
    }
}
