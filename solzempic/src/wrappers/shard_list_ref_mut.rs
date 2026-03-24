//! Mutable shard list reference for singly-linked navigation.
//!
//! This module provides [`ShardListRefMut`], a container for managing
//! singly-linked shard lists with mutable access for insert/remove operations.

use core::ptr;

use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;

use crate::{Framework, Loadable};

use super::account_ref_mut::AccountRefMut;
use super::shard_list_ref::ShardListNode;

/// Context for mutable singly-linked shard navigation.
///
/// `ShardListRefMut` manages a current shard and optionally its next neighbor
/// with mutable access. This supports insert/remove operations in the linked list.
///
/// # Deduplication
///
/// When the same account is passed for both positions (e.g., current == next),
/// this struct automatically deduplicates to avoid undefined behavior from
/// aliased mutable references.
///
/// # Use Cases
///
/// - **Order placement**: Inserting into single-side bid/ask chains
/// - **Order cancellation**: Removing from linked list
/// - **Rebalancing**: Moving orders between shards
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
/// use solzempic::ShardListRefMut;
///
/// // Load current and next shards for insertion
/// let mut shards: ShardListRefMut<LimitBidShard> = ShardListRefMut::with_next(
///     &accounts[0],  // current
///     &accounts[1],  // next
/// )?;
///
/// // Insert a new shard between current and next
/// shards.current_mut().next_shard = new_shard_address;
/// ```
///
/// # Performance
///
/// Loading loads each unique account once (~50 CUs each).
/// Duplicate accounts are detected and only loaded once.
pub struct ShardListRefMut<'a, T: ShardListNode, F: Framework> {
    /// The current shard (always loaded).
    current: AccountRefMut<'a, T, F>,
    /// The next shard - may alias current if same account.
    next_ref: NextRef<'a, T, F>,
}

/// Next shard reference - either owned or aliasing current
enum NextRef<'a, T: Loadable, F: Framework> {
    None,
    Owned(AccountRefMut<'a, T, F>),
    AliasCurrent, // Same as current
}

impl<'a, T: ShardListNode, F: Framework> ShardListRefMut<'a, T, F> {
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
            current: AccountRefMut::load(current_info)?,
            next_ref: NextRef::None,
        })
    }

    /// Create a context with current and next shards loaded.
    ///
    /// Automatically deduplicates if the same account is passed for both.
    ///
    /// # Arguments
    ///
    /// * `current_info` - The current shard's AccountInfo
    /// * `next_info` - The next shard's AccountInfo
    ///
    /// # Errors
    ///
    /// Returns an error if either unique account fails validation.
    #[inline]
    pub fn with_next(
        current_info: &'a AccountView,
        next_info: &'a AccountView,
    ) -> Result<Self, ProgramError> {
        let current = AccountRefMut::load(current_info)?;

        // Check if next is same as current
        let current_next_same = ptr::eq(current_info, next_info);
        let next_ref = if current_next_same {
            NextRef::AliasCurrent
        } else {
            NextRef::Owned(AccountRefMut::load(next_info)?)
        };

        Ok(Self { current, next_ref })
    }

    /// Try to create a context, returning `None` if any account is invalid.
    ///
    /// This is useful for optional next shards that may not exist yet.
    #[inline]
    pub fn try_new(current_info: &'a AccountView) -> Option<Self> {
        Some(Self {
            current: AccountRefMut::try_load(current_info)?,
            next_ref: NextRef::None,
        })
    }

    /// Try to create a context with next, returning `None` if any account is invalid.
    #[inline]
    pub fn try_with_next(
        current_info: &'a AccountView,
        next_info: &'a AccountView,
    ) -> Option<Self> {
        let current = AccountRefMut::try_load(current_info)?;

        let current_next_same = ptr::eq(current_info, next_info);
        let next_ref = if current_next_same {
            NextRef::AliasCurrent
        } else {
            NextRef::Owned(AccountRefMut::try_load(next_info)?)
        };

        Some(Self { current, next_ref })
    }

    /// Create a context from already-loaded shard wrappers.
    ///
    /// **Warning**: Does not perform deduplication. Caller must ensure
    /// accounts are different if passing two AccountRefMut.
    #[inline]
    pub fn from_loaded(
        current: AccountRefMut<'a, T, F>,
        next: Option<AccountRefMut<'a, T, F>>,
    ) -> Self {
        Self {
            current,
            next_ref: match next {
                Some(n) => NextRef::Owned(n),
                None => NextRef::None,
            },
        }
    }

    /// Get the current AccountRefMut reference.
    #[inline]
    pub fn current_ref(&self) -> &AccountRefMut<'a, T, F> {
        &self.current
    }

    /// Get the current AccountRefMut mutably.
    #[inline]
    pub fn current_ref_mut(&mut self) -> &mut AccountRefMut<'a, T, F> {
        &mut self.current
    }

    /// Get the next AccountRefMut reference (if loaded).
    #[inline]
    pub fn next_ref(&self) -> Option<&AccountRefMut<'a, T, F>> {
        match &self.next_ref {
            NextRef::None => None,
            NextRef::Owned(acct) => Some(acct),
            NextRef::AliasCurrent => Some(&self.current),
        }
    }

    /// Get the next AccountRefMut mutably (if loaded).
    #[inline]
    pub fn next_ref_mut(&mut self) -> Option<&mut AccountRefMut<'a, T, F>> {
        match &mut self.next_ref {
            NextRef::None => None,
            NextRef::Owned(acct) => Some(acct),
            NextRef::AliasCurrent => Some(&mut self.current),
        }
    }

    /// Get the address of the current shard.
    #[inline]
    pub fn current_address(&self) -> &Address {
        self.current.address()
    }

    /// Get the address of the next shard (if loaded).
    #[inline]
    pub fn next_address(&self) -> Option<&Address> {
        self.next_ref().map(|n| n.address())
    }

    /// Get read-only access to the current shard's data.
    #[inline]
    pub fn current(&self) -> &T {
        self.current.get()
    }

    /// Get read-only access to the next shard's data (if loaded).
    #[inline]
    pub fn next(&self) -> Option<&T> {
        self.next_ref().map(|n| n.get())
    }

    /// Get mutable access to the current shard's data.
    #[inline]
    pub fn current_mut(&mut self) -> &mut T {
        self.current.get_mut()
    }

    /// Get mutable access to the next shard's data (if loaded).
    #[inline]
    pub fn next_mut(&mut self) -> Option<&mut T> {
        match &mut self.next_ref {
            NextRef::None => None,
            NextRef::Owned(acct) => Some(acct.get_mut()),
            NextRef::AliasCurrent => Some(self.current.get_mut()),
        }
    }

    /// Get mutable access to both shards simultaneously.
    ///
    /// Returns `(current, next)` where next may be None.
    /// When accounts are deduplicated, both references point to the same data.
    ///
    /// # Safety
    ///
    /// Uses raw pointers to allow intentional aliasing when the same account
    /// is passed for both positions.
    #[inline]
    pub fn all_mut(&mut self) -> (&mut T, Option<&mut T>) {
        let current_ptr = self.current.get_mut() as *mut T;

        let next = match &mut self.next_ref {
            NextRef::None => None,
            NextRef::Owned(acct) => Some(acct.get_mut() as *mut T),
            NextRef::AliasCurrent => Some(current_ptr),
        };

        unsafe { (&mut *current_ptr, next.map(|p| &mut *p)) }
    }

    /// Get mutable access to both shards' raw data slices.
    ///
    /// Returns `(current_data, next_data)` where next may be None.
    #[inline]
    pub fn all_data_mut(&mut self) -> (&mut [u8], Option<&mut [u8]>) {
        let current_ptr = self.current.data_mut() as *mut [u8];

        let next = match &mut self.next_ref {
            NextRef::None | NextRef::AliasCurrent => None,
            NextRef::Owned(acct) => Some(acct.data_mut() as *mut [u8]),
        };

        unsafe { (&mut *current_ptr, next.map(|p| &mut *p)) }
    }

    /// Check if this is the terminal shard (no successor in list).
    #[inline]
    pub fn is_terminal(&self) -> bool {
        self.current.get().is_terminal()
    }

    /// Check if the next shard is loaded.
    #[inline]
    pub fn has_next(&self) -> bool {
        !matches!(self.next_ref, NextRef::None)
    }

    /// Link a new shard after the current one.
    ///
    /// Updates current's next pointer to point to `new_shard_address`,
    /// and the new shard should point to whatever current was pointing to.
    ///
    /// # Arguments
    ///
    /// * `new_shard_address` - Address of the new shard to insert
    ///
    /// # Returns
    ///
    /// The old next address (what the new shard should point to).
    #[inline]
    pub fn insert_after(&mut self, new_shard_address: Address) -> Address {
        let old_next = *self.current.get().next_shard();
        *self.current.get_mut().next_shard_mut() = new_shard_address;
        old_next
    }

    /// Remove the next shard from the list.
    ///
    /// Updates current's next pointer to skip over the next shard.
    /// The next shard must be loaded for this to work.
    ///
    /// # Panics
    ///
    /// Panics if next shard is not loaded.
    #[inline]
    pub fn remove_next(&mut self) {
        let next_next = match &self.next_ref {
            NextRef::None => panic!("cannot remove_next without next loaded"),
            NextRef::Owned(acct) => *acct.get().next_shard(),
            NextRef::AliasCurrent => panic!("cannot remove_next when aliased"),
        };
        *self.current.get_mut().next_shard_mut() = next_next;
    }
}

// ============================================================================
// From Implementations
// ============================================================================

impl<'a, T: ShardListNode, F: Framework>
    From<(AccountRefMut<'a, T, F>, Option<AccountRefMut<'a, T, F>>)> for ShardListRefMut<'a, T, F>
{
    /// Create a ShardListRefMut from a tuple of (current, optional next).
    ///
    /// # Example
    /// ```ignore
    /// let shards: ShardListRefMut<_> = (current, Some(next)).into();
    /// let shards: ShardListRefMut<_> = (current, None).into();
    /// ```
    #[inline]
    fn from((current, next): (AccountRefMut<'a, T, F>, Option<AccountRefMut<'a, T, F>>)) -> Self {
        Self::from_loaded(current, next)
    }
}

impl<'a, T: ShardListNode, F: Framework> From<(AccountRefMut<'a, T, F>, AccountRefMut<'a, T, F>)>
    for ShardListRefMut<'a, T, F>
{
    /// Create a ShardListRefMut from a tuple of (current, next).
    ///
    /// # Example
    /// ```ignore
    /// let shards: ShardListRefMut<_> = (current, next).into();
    /// ```
    #[inline]
    fn from((current, next): (AccountRefMut<'a, T, F>, AccountRefMut<'a, T, F>)) -> Self {
        Self::from_loaded(current, Some(next))
    }
}
