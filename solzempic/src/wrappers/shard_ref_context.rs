//! Read-only shard reference context for triplet navigation.
//!
//! This module provides [`ShardRefContext`], a container for managing three
//! related shard accounts (previous, current, next) with read-only access.

use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;

use crate::{Framework, Loadable};

use super::account_ref::AccountRef;

/// Context holding read-only references to a triplet of shards.
///
/// `ShardRefContext` manages three [`AccountRef`] references for sharded
/// data structures where operations need to read neighboring shards.
/// This is commonly used for:
///
/// - **Price lookups**: Finding best price across shard boundaries
/// - **Order matching**: Reading orders from multiple shards
/// - **Validation**: Checking shard invariants without modification
///
/// # IDL Generation
///
/// When used in instruction structs, all three accounts are marked as read-only
/// (not writable) in the generated IDL.
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
/// // Load a shard triplet for reading
/// let shards: ShardRefContext<OrderShard> = ShardRefContext::new(
///     &accounts[0],  // prev shard
///     &accounts[1],  // current shard
///     &accounts[2],  // next shard
/// )?;
///
/// // Read from all three shards
/// let prev_count = shards.prev().order_count;
/// let current_count = shards.current().order_count;
/// let next_count = shards.next().order_count;
/// ```
///
/// # Performance
///
/// Loading a `ShardRefContext` loads all three shards upfront (~150 CUs total).
/// This is efficient when you know you'll need to read from neighboring shards.
///
/// If you need mutable access, use [`ShardRefMutContext`](super::ShardRefMutContext) instead.
/// If you only need a single shard, use [`AccountRef`] directly.
pub struct ShardRefContext<'a, T: Loadable, F: Framework> {
    /// The previous shard in the linked structure.
    pub prev: AccountRef<'a, T, F>,
    /// The current (primary) shard being operated on.
    pub current: AccountRef<'a, T, F>,
    /// The next shard in the linked structure.
    pub next: AccountRef<'a, T, F>,
}

impl<'a, T: Loadable, F: Framework> ShardRefContext<'a, T, F> {
    /// Create a new shard context by loading three account infos.
    ///
    /// All three accounts must be already initialized with valid data of type `T`.
    /// Each account is loaded as an [`AccountRef`] with full validation.
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
    /// wrong discriminator, etc.).
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
            prev: AccountRef::load(prev_info)?,
            current: AccountRef::load(current_info)?,
            next: AccountRef::load(next_info)?,
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
    /// let prev = AccountRef::load(&accounts[0])?;
    /// let current = AccountRef::load(&accounts[1])?;
    /// let next = AccountRef::load(&accounts[2])?;
    ///
    /// let shards = ShardRefContext::from_loaded(prev, current, next);
    /// ```
    #[inline]
    pub fn from_loaded(
        prev: AccountRef<'a, T, F>,
        current: AccountRef<'a, T, F>,
        next: AccountRef<'a, T, F>,
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

    /// Get read-only access to the current shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let count = shards.current().order_count;
    /// ```
    #[inline]
    pub fn current(&self) -> &T {
        self.current.get()
    }

    /// Get read-only access to the previous shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let prev_max_price = shards.prev().max_price;
    /// ```
    #[inline]
    pub fn prev(&self) -> &T {
        self.prev.get()
    }

    /// Get read-only access to the next shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let next_min_price = shards.next().min_price;
    /// ```
    #[inline]
    pub fn next(&self) -> &T {
        self.next.get()
    }

    /// Get read-only access to all three shards simultaneously.
    ///
    /// Returns a tuple of `(prev, current, next)` references.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (prev, current, next) = shards.all();
    ///
    /// // Check price continuity across shards
    /// assert!(prev.max_price <= current.min_price);
    /// assert!(current.max_price <= next.min_price);
    /// ```
    #[inline]
    pub fn all(&self) -> (&T, &T, &T) {
        (
            self.prev.get(),
            self.current.get(),
            self.next.get(),
        )
    }
}
