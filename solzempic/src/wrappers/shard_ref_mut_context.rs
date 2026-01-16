//! Mutable shard reference context for triplet navigation.
//!
//! This module provides [`ShardRefMutContext`], a container for managing three
//! related shard accounts (low, current, high) with mutable access.

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
/// # Invariant
///
/// All three shards are always valid and initialized. The sharding system
/// maintains the invariant that:
///
/// - Markets always have at least 3 shards
/// - Every shard has valid `low` and `high` neighbors (circular linked list)
/// - Edge-case handling is eliminated by this guarantee
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
/// // Access all three shards mutably
/// let (low, current, high) = shards.all_mut();
/// // ... perform rebalancing logic
/// ```
///
/// # Performance
///
/// Loading a `ShardRefMutContext` loads all three shards upfront (~150 CUs total).
/// This is efficient when you know you'll need mutable access to neighboring shards.
///
/// If you only need read-only access, use [`ShardRefContext`](super::ShardRefContext) instead.
/// If you only need a single shard, use [`AccountRefMut`] directly.
pub struct ShardRefMutContext<'a, T: Loadable, F: Framework> {
    /// The low shard in the linked structure.
    pub low: AccountRefMut<'a, T, F>,
    /// The current (primary) shard being operated on.
    pub current: AccountRefMut<'a, T, F>,
    /// The high shard in the linked structure.
    pub high: AccountRefMut<'a, T, F>,
}

impl<'a, T: Loadable, F: Framework> ShardRefMutContext<'a, T, F> {
    /// Create a new shard context by loading three account infos.
    ///
    /// All three accounts must be already initialized with valid data of type `T`.
    /// Each account is loaded as an [`AccountRefMut`] with full validation.
    ///
    /// # Arguments
    ///
    /// * `low_info` - The low shard's AccountInfo
    /// * `current_info` - The current (primary) shard's AccountInfo
    /// * `high_info` - The high shard's AccountInfo
    ///
    /// # Errors
    ///
    /// Returns an error if any of the three accounts fail validation (wrong owner,
    /// not writable, wrong discriminator, etc.).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let shards = ShardRefMutContext::<OrderShard>::new(
    ///     &accounts[0],  // low
    ///     &accounts[1],  // current
    ///     &accounts[2],  // high
    /// )?;
    /// ```
    #[inline]
    pub fn new(
        low_info: &'a AccountView,
        current_info: &'a AccountView,
        high_info: &'a AccountView,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            low: AccountRefMut::load(low_info)?,
            current: AccountRefMut::load(current_info)?,
            high: AccountRefMut::load(high_info)?,
        })
    }

    /// Try to create a shard context, returning `None` if any account is invalid.
    ///
    /// This is useful for optional opposite-side shards that may not exist yet.
    /// For example, when placing a bid order, the ask-side shards may not be
    /// initialized if no ask orders have been placed. In this case, crossing
    /// check can be skipped for that liquidity source.
    ///
    /// # Arguments
    ///
    /// * `low_info` - The low shard's AccountInfo
    /// * `current_info` - The current (primary) shard's AccountInfo
    /// * `high_info` - The high shard's AccountInfo
    ///
    /// # Returns
    ///
    /// `Some(Self)` if all three accounts are valid and initialized, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(mut opposite_shards) = ShardRefMutContext::<OrderShard>::try_new(
    ///     &accounts[0],  // low
    ///     &accounts[1],  // current
    ///     &accounts[2],  // high
    /// ) {
    ///     // Opposite shards exist, check for crossing
    ///     if let Some(best_price) = opposite_shards.current().best_price() {
    ///         // ... crossing check
    ///     }
    /// } else {
    ///     // No opposite shards, no crossing possible
    /// }
    /// ```
    #[inline]
    pub fn try_new(
        low_info: &'a AccountView,
        current_info: &'a AccountView,
        high_info: &'a AccountView,
    ) -> Option<Self> {
        Some(Self {
            low: AccountRefMut::try_load(low_info)?,
            current: AccountRefMut::try_load(current_info)?,
            high: AccountRefMut::try_load(high_info)?,
        })
    }

    /// Create a context from already-loaded shard wrappers.
    ///
    /// Use this when you've already loaded the shards individually and want
    /// to combine them into a context. This avoids re-validating accounts.
    ///
    /// # Arguments
    ///
    /// * `low` - Already-loaded low shard
    /// * `current` - Already-loaded current shard
    /// * `high` - Already-loaded high shard
    ///
    /// # Example
    ///
    /// ```ignore
    /// let low = AccountRefMut::load(&accounts[0])?;
    /// let current = AccountRefMut::load(&accounts[1])?;
    /// let high = AccountRefMut::load(&accounts[2])?;
    ///
    /// let shards = ShardRefMutContext::from_loaded(low, current, high);
    /// ```
    #[inline]
    pub fn from_loaded(
        low: AccountRefMut<'a, T, F>,
        current: AccountRefMut<'a, T, F>,
        high: AccountRefMut<'a, T, F>,
    ) -> Self {
        Self { low, current, high }
    }

    /// Get the address of the current shard.
    #[inline]
    pub fn current_address(&self) -> &Address {
        self.current.address()
    }

    /// Get the address of the low shard.
    #[inline]
    pub fn low_address(&self) -> &Address {
        self.low.address()
    }

    /// Get the address of the high shard.
    #[inline]
    pub fn high_address(&self) -> &Address {
        self.high.address()
    }

    /// Get read-only access to the current shard's data.
    #[inline]
    pub fn current(&self) -> &T {
        self.current.get()
    }

    /// Get read-only access to the low shard's data.
    #[inline]
    pub fn low(&self) -> &T {
        self.low.get()
    }

    /// Get read-only access to the high shard's data.
    #[inline]
    pub fn high(&self) -> &T {
        self.high.get()
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
        self.current.get_mut()
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
        self.high.get_mut()
    }

    /// Get mutable access to all three shards simultaneously.
    ///
    /// Returns a tuple of `(low, current, high)` mutable references.
    /// This is useful for operations that need to update multiple shards
    /// atomically, such as rebalancing or inserting into a linked structure.
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
    #[inline]
    pub fn all_mut(&mut self) -> (&mut T, &mut T, &mut T) {
        (
            self.low.get_mut(),
            self.current.get_mut(),
            self.high.get_mut(),
        )
    }
}
