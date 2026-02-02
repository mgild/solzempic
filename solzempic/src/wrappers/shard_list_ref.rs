//! Read-only shard list reference for singly-linked navigation.
//!
//! This module provides [`ShardListRef`], a simple container for navigating
//! singly-linked shard lists where each shard only needs to know its successor.

use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;

use crate::{Framework, Loadable};

use super::account_ref::AccountRef;

/// Trait for shard types in a singly-linked list.
///
/// Implement this trait for shard header types that participate in a
/// singly-linked list structure. Each shard knows only its next neighbor.
///
/// # Example
///
/// ```ignore
/// use solzempic::ShardListNode;
///
/// impl ShardListNode for LimitBidShardHeader {
///     fn next_shard(&self) -> &Address {
///         &self.next_shard
///     }
///
///     fn next_shard_mut(&mut self) -> &mut Address {
///         &mut self.next_shard
///     }
/// }
/// ```
pub trait ShardListNode: Loadable {
    /// Get the address of the next shard in the list.
    ///
    /// Returns `Address::default()` (all zeros) if this is the terminal shard.
    fn next_shard(&self) -> &Address;

    /// Get a mutable reference to the next shard address.
    fn next_shard_mut(&mut self) -> &mut Address;

    /// Check if this is the terminal shard (end of list).
    #[inline]
    fn is_terminal(&self) -> bool {
        *self.next_shard() == Address::default()
    }
}

/// Context for read-only singly-linked shard navigation.
///
/// `ShardListRef` manages a current shard and optionally its next neighbor
/// for singly-linked shard structures. This is simpler than [`ShardRefContext`]
/// which manages triplets (low, current, high).
///
/// # Use Cases
///
/// - **Single-side orderbook traversal**: Following bid or ask chain separately
/// - **Free list iteration**: Traversing evicted shards
/// - **Amount-ordered iteration**: Following CLMM positions by size
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the borrowed AccountInfo references
/// * `T` - The shard data type (must implement [`ShardListNode`])
/// * `F` - The framework type (must implement [`Framework`](crate::Framework))
///
/// # Example
///
/// ```ignore
/// use solzempic::ShardListRef;
///
/// // Load current shard for reading
/// let shards: ShardListRef<LimitBidShard> = ShardListRef::new(&accounts[0])?;
///
/// // Check if there's a next shard
/// if !shards.current().is_terminal() {
///     let next_addr = shards.current().next_shard();
///     // Load next shard if needed...
/// }
/// ```
///
/// # Performance
///
/// Loading a `ShardListRef` loads only the current shard (~50 CUs).
/// Use [`with_next`](Self::with_next) to also load the next shard when needed.
pub struct ShardListRef<'a, T: ShardListNode, F: Framework> {
    /// The current shard being operated on.
    current: AccountRef<'a, T, F>,
    /// The next shard (if loaded).
    next: Option<AccountRef<'a, T, F>>,
}

impl<'a, T: ShardListNode, F: Framework> ShardListRef<'a, T, F> {
    /// Create a new shard list context with just the current shard.
    ///
    /// # Arguments
    ///
    /// * `current_info` - The current shard's AccountInfo
    ///
    /// # Errors
    ///
    /// Returns an error if the account fails validation.
    #[inline]
    pub fn new(current_info: &'a AccountView) -> Result<Self, ProgramError> {
        Ok(Self {
            current: AccountRef::load(current_info)?,
            next: None,
        })
    }

    /// Create a context with current and next shards loaded.
    ///
    /// Use this when you need to read from both the current shard and its
    /// successor in the list.
    ///
    /// # Arguments
    ///
    /// * `current_info` - The current shard's AccountInfo
    /// * `next_info` - The next shard's AccountInfo
    ///
    /// # Errors
    ///
    /// Returns an error if either account fails validation.
    #[inline]
    pub fn with_next(
        current_info: &'a AccountView,
        next_info: &'a AccountView,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            current: AccountRef::load(current_info)?,
            next: Some(AccountRef::load(next_info)?),
        })
    }

    /// Create a context from already-loaded shard wrappers.
    #[inline]
    pub fn from_loaded(current: AccountRef<'a, T, F>, next: Option<AccountRef<'a, T, F>>) -> Self {
        Self { current, next }
    }

    /// Get the address of the current shard.
    #[inline]
    pub fn current_address(&self) -> &Address {
        self.current.address()
    }

    /// Get the address of the next shard (if loaded).
    #[inline]
    pub fn next_address(&self) -> Option<&Address> {
        self.next.as_ref().map(|n| n.address())
    }

    /// Get read-only access to the current shard's data.
    #[inline]
    pub fn current(&self) -> &T {
        self.current.get()
    }

    /// Get read-only access to the next shard's data (if loaded).
    #[inline]
    pub fn next(&self) -> Option<&T> {
        self.next.as_ref().map(|n| n.get())
    }

    /// Get the underlying AccountRef for the current shard.
    #[inline]
    pub fn current_ref(&self) -> &AccountRef<'a, T, F> {
        &self.current
    }

    /// Get the underlying AccountRef for the next shard (if loaded).
    #[inline]
    pub fn next_ref(&self) -> Option<&AccountRef<'a, T, F>> {
        self.next.as_ref()
    }

    /// Check if this is the terminal shard (no successor).
    #[inline]
    pub fn is_terminal(&self) -> bool {
        self.current.get().is_terminal()
    }

    /// Check if the next shard is loaded.
    #[inline]
    pub fn has_next(&self) -> bool {
        self.next.is_some()
    }
}
