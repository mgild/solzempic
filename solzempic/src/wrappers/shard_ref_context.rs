//! Shard reference context for triplet navigation.
//!
//! This module provides [`ShardRefContext`], a container for managing three
//! related shard accounts (previous, current, next) that form a navigable
//! chain in sharded data structures.

use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;

use crate::{Framework, Loadable};

use super::account_ref_mut::AccountRefMut;

/// Context holding writable references to a triplet of shards.
///
/// `ShardRefContext` manages three [`AccountRefMut`] references for sharded
/// data structures where operations may need to access neighboring shards.
/// This is commonly used for:
///
/// - **Orderbooks**: Orders may need to move between price-range shards
/// - **Linked lists**: Insertions/deletions update prev/next pointers
/// - **Rebalancing**: Moving data between under/over-utilized shards
///
/// # Invariant
///
/// All three shards are always valid and initialized. The sharding system
/// maintains the invariant that:
///
/// - Markets always have at least 3 shards
/// - Every shard has valid `prev` and `next` neighbors (circular linked list)
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
/// use solzempic::ShardRefContext;
///
/// // Load a shard triplet for an order operation
/// let mut shards: ShardRefContext<OrderShard> = ShardRefContext::new(
///     &accounts[0],  // prev shard
///     &accounts[1],  // current shard
///     &accounts[2],  // next shard
/// )?;
///
/// // Access all three shards mutably
/// let (prev, current, next) = shards.all_mut();
/// // ... perform rebalancing logic
/// ```
///
/// # Performance
///
/// Loading a `ShardRefContext` loads all three shards upfront (~150 CUs total).
/// This is efficient when you know you'll need access to neighboring shards.
///
/// If you only need a single shard, use [`AccountRefMut`] directly instead.
pub struct ShardRefContext<'a, T: Loadable, F: Framework> {
    /// The previous shard in the linked structure.
    pub prev: AccountRefMut<'a, T, F>,
    /// The current (primary) shard being operated on.
    pub current: AccountRefMut<'a, T, F>,
    /// The next shard in the linked structure.
    pub next: AccountRefMut<'a, T, F>,
}

impl<'a, T: Loadable, F: Framework> ShardRefContext<'a, T, F> {
    /// Create a new shard context by loading three account infos.
    ///
    /// All three accounts must be already initialized with valid data of type `T`.
    /// Each account is loaded as an [`AccountRefMut`] with full validation.
    ///
    /// # Arguments
    ///
    /// * `prev_info` - The previous shard's AccountInfo
    /// * `current_info` - The current (primary) shard's AccountInfo
    /// * `next_info` - The next shard's AccountInfo
    ///
    /// # Errors
    ///
    /// Returns an error if any of the three accounts fail validation (wrong owner,
    /// not writable, wrong discriminator, etc.).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let shards = ShardRefContext::<OrderShard>::new(
    ///     &accounts[0],  // prev
    ///     &accounts[1],  // current
    ///     &accounts[2],  // next
    /// )?;
    /// ```
    #[inline]
    pub fn new(
        prev_info: &'a AccountView,
        current_info: &'a AccountView,
        next_info: &'a AccountView,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            prev: AccountRefMut::load(prev_info)?,
            current: AccountRefMut::load(current_info)?,
            next: AccountRefMut::load(next_info)?,
        })
    }

    /// Create a context from already-loaded shard wrappers.
    ///
    /// Use this when you've already loaded the shards individually and want
    /// to combine them into a context. This avoids re-validating accounts.
    ///
    /// # Arguments
    ///
    /// * `prev` - Already-loaded previous shard
    /// * `current` - Already-loaded current shard
    /// * `next` - Already-loaded next shard
    ///
    /// # Example
    ///
    /// ```ignore
    /// let prev = AccountRefMut::load(&accounts[0])?;
    /// let current = AccountRefMut::load(&accounts[1])?;
    /// let next = AccountRefMut::load(&accounts[2])?;
    ///
    /// let shards = ShardRefContext::from_loaded(prev, current, next);
    /// ```
    #[inline]
    pub fn from_loaded(
        prev: AccountRefMut<'a, T, F>,
        current: AccountRefMut<'a, T, F>,
        next: AccountRefMut<'a, T, F>,
    ) -> Self {
        Self { prev, current, next }
    }

    /// Get the address of the current shard.
    #[inline]
    pub fn current_address(&self) -> &Address {
        self.current.address()
    }

    /// Get the address of the previous shard.
    #[inline]
    pub fn prev_address(&self) -> &Address {
        self.prev.address()
    }

    /// Get the address of the next shard.
    #[inline]
    pub fn next_address(&self) -> &Address {
        self.next.address()
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

    /// Get mutable access to the previous shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// shards.prev_mut().next_shard = *new_shard_key;
    /// ```
    #[inline]
    pub fn prev_mut(&mut self) -> &mut T {
        self.prev.get_mut()
    }

    /// Get mutable access to the next shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// shards.next_mut().prev_shard = *new_shard_key;
    /// ```
    #[inline]
    pub fn next_mut(&mut self) -> &mut T {
        self.next.get_mut()
    }

    /// Get mutable access to all three shards simultaneously.
    ///
    /// Returns a tuple of `(prev, current, next)` mutable references.
    /// This is useful for operations that need to update multiple shards
    /// atomically, such as rebalancing or inserting into a linked structure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (prev, current, next) = shards.all_mut();
    ///
    /// // Move an order from current to next shard
    /// let order = current.remove_order(order_idx);
    /// next.insert_order(order);
    ///
    /// // Update linked list pointers
    /// prev.next_shard = *current_key;
    /// next.prev_shard = *current_key;
    /// ```
    #[inline]
    pub fn all_mut(&mut self) -> (&mut T, &mut T, &mut T) {
        (
            self.prev.get_mut(),
            self.current.get_mut(),
            self.next.get_mut(),
        )
    }
}
