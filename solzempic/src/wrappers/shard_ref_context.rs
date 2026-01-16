//! Read-only shard reference context for triplet navigation.
//!
//! This module provides [`ShardRefContext`], a container for managing three
//! related shard accounts (low, current, high) with read-only access.

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
/// (not writable) in the generated IDL with names: `{field}_low_shard`, `{field}_current_shard`, `{field}_high_shard`.
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
/// use solzempic::ShardRefContext;
///
/// // Load a shard triplet for reading
/// let shards: ShardRefContext<OrderShard> = ShardRefContext::new(
///     &accounts[0],  // low shard
///     &accounts[1],  // current shard
///     &accounts[2],  // high shard
/// )?;
///
/// // Read from all three shards
/// let low_count = shards.low().order_count;
/// let current_count = shards.current().order_count;
/// let high_count = shards.high().order_count;
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
    /// The low shard in the linked structure.
    pub low: AccountRef<'a, T, F>,
    /// The current (primary) shard being operated on.
    pub current: AccountRef<'a, T, F>,
    /// The high shard in the linked structure.
    pub high: AccountRef<'a, T, F>,
}

impl<'a, T: Loadable, F: Framework> ShardRefContext<'a, T, F> {
    /// Create a new shard context by loading three account infos.
    ///
    /// All three accounts must be already initialized with valid data of type `T`.
    /// Each account is loaded as an [`AccountRef`] with full validation.
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
    /// wrong discriminator, etc.).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let shards = ShardRefContext::<OrderShard>::new(
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
            low: AccountRef::load(low_info)?,
            current: AccountRef::load(current_info)?,
            high: AccountRef::load(high_info)?,
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
    /// let low = AccountRef::load(&accounts[0])?;
    /// let current = AccountRef::load(&accounts[1])?;
    /// let high = AccountRef::load(&accounts[2])?;
    ///
    /// let shards = ShardRefContext::from_loaded(low, current, high);
    /// ```
    #[inline]
    pub fn from_loaded(
        low: AccountRef<'a, T, F>,
        current: AccountRef<'a, T, F>,
        high: AccountRef<'a, T, F>,
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

    /// Get read-only access to the low shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let low_max_price = shards.low().max_price;
    /// ```
    #[inline]
    pub fn low(&self) -> &T {
        self.low.get()
    }

    /// Get read-only access to the high shard's data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let high_min_price = shards.high().min_price;
    /// ```
    #[inline]
    pub fn high(&self) -> &T {
        self.high.get()
    }

    /// Get read-only access to all three shards simultaneously.
    ///
    /// Returns a tuple of `(low, current, high)` references.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (low, current, high) = shards.all();
    ///
    /// // Check price continuity across shards
    /// assert!(low.max_price <= current.min_price);
    /// assert!(current.max_price <= high.min_price);
    /// ```
    #[inline]
    pub fn all(&self) -> (&T, &T, &T) {
        (
            self.low.get(),
            self.current.get(),
            self.high.get(),
        )
    }
}
