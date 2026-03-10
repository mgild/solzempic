//! Variable-length account group list, sentinel-terminated.
//!
//! [`AccountVec`] allows instructions to accept a variable number of
//! structured account groups without encoding the count in instruction data.
//! Instead, clients either append a sentinel account (the program ID) to
//! signal end-of-list, or simply stop adding accounts (the parser handles
//! both styles transparently).
//!
//! ## Design
//!
//! Groups are parsed left-to-right from a tail slice of the `accounts` array.
//! Each group is described by a type `T` implementing [`FromAccountSlice`], which
//! declares how many accounts it consumes (`ACCOUNTS_PER_GROUP`) and how to load
//! them.  Parsing terminates when:
//!
//! - The remaining slice is empty, **or**
//! - The first account in the remaining slice has a key equal to `sentinel`
//!   (typically the program ID, which can never appear as a real account owner).
//!
//! Stopping on the sentinel rather than on a count makes it easy to append
//! zero groups (the sentinel is the only variable account) and allows future
//! instructions to place fixed accounts *after* the vec by using a known
//! sentinel address as a boundary marker.
//!
//! ## Client-side convention
//!
//! On the client, assemble the instruction accounts as:
//!
//! ```ignore
//! // TypeScript / JS client example:
//! const accounts = [
//!     ...fixedAccounts,
//!     ...group1.toAccountMetas(),   // ACCOUNTS_PER_GROUP accounts
//!     ...group2.toAccountMetas(),   // ACCOUNTS_PER_GROUP accounts
//!     { pubkey: PROGRAM_ID, isSigner: false, isWritable: false }, // sentinel
//! ];
//! ```
//!
//! The sentinel is **optional** when the variable-length list is the last
//! set of accounts in the instruction — parsing stops at end-of-slice just
//! as gracefully.
//!
//! ## Example
//!
//! ```ignore
//! use solzempic::{AccountVec, FromAccountSlice, AccountRef};
//! use pinocchio::{error::ProgramError, AccountView};
//!
//! /// One group: a trader account + their two token accounts.
//! pub struct TraderGroup<'a> {
//!     pub trader: AccountRef<'a, Trader>,
//!     pub token_a: TokenAccountRefMut<'a>,
//!     pub token_b: TokenAccountRefMut<'a>,
//! }
//!
//! impl<'a> FromAccountSlice<'a> for TraderGroup<'a> {
//!     const ACCOUNTS_PER_GROUP: usize = 3;
//!
//!     fn from_slice(accounts: &'a [AccountView]) -> Result<Self, ProgramError> {
//!         Ok(Self {
//!             trader:  AccountRef::load(&accounts[0])?,
//!             token_a: TokenAccountRefMut::load(&accounts[1])?,
//!             token_b: TokenAccountRefMut::load(&accounts[2])?,
//!         })
//!     }
//! }
//!
//! // Inside your instruction's build():
//! const FIXED: usize = 9;
//! let groups = AccountVec::<TraderGroup>::parse(&accounts[FIXED..], program_id)?;
//! for group in groups.iter() {
//!     // process each group ...
//! }
//! ```

use alloc::vec::Vec;
use core::marker::PhantomData;

use pinocchio::{error::ProgramError, AccountView};
use solana_address::{address_eq, Address};

// ============================================================================
// FromAccountSlice trait
// ============================================================================

/// Trait for types that can be parsed from a fixed-size prefix of an account slice.
///
/// This is the on-chain counterpart of the client-side `ToAccountMetas` pattern:
/// the client serialises a group into a list of `AccountMeta`s; the program
/// deserialises the same group from the corresponding `AccountView` slice.
///
/// # Implementation contract
///
/// - `ACCOUNTS_PER_GROUP` must equal the number of `AccountView` entries
///   that `from_slice` reads.  The caller guarantees `accounts.len() >=
///   ACCOUNTS_PER_GROUP` before calling `from_slice`.
/// - `from_slice` should return `Err` if any account fails validation
///   (wrong owner, not writable, wrong discriminator, …).
///
/// # Example
///
/// ```ignore
/// pub struct MyGroup<'a> {
///     pub user:  AccountRef<'a, User>,
///     pub vault: TokenAccountRefMut<'a>,
/// }
///
/// impl<'a> FromAccountSlice<'a> for MyGroup<'a> {
///     const ACCOUNTS_PER_GROUP: usize = 2;
///
///     fn from_slice(accounts: &'a [AccountView]) -> Result<Self, ProgramError> {
///         Ok(Self {
///             user:  AccountRef::load(&accounts[0])?,
///             vault: TokenAccountRefMut::load(&accounts[1])?,
///         })
///     }
/// }
/// ```
pub trait FromAccountSlice<'a>: Sized {
    /// Number of `AccountView`s consumed per group.
    ///
    /// The parser advances the slice by exactly this many positions after each
    /// successful [`from_slice`](Self::from_slice) call.
    const ACCOUNTS_PER_GROUP: usize;

    /// Parse one group from the first `ACCOUNTS_PER_GROUP` entries of `accounts`.
    ///
    /// The caller guarantees `accounts.len() >= ACCOUNTS_PER_GROUP`.
    ///
    /// # Errors
    ///
    /// Return `Err` if any account in the group fails validation.
    fn from_slice(accounts: &'a [AccountView]) -> Result<Self, ProgramError>;
}

// ============================================================================
// AccountVec
// ============================================================================

/// Variable-length list of typed account groups, sentinel-terminated.
///
/// Parses zero or more groups of `T::ACCOUNTS_PER_GROUP` accounts from a
/// tail slice of an instruction's account list.  Parsing stops when:
///
/// - The remaining slice is empty, **or**
/// - The first account in the remaining slice has a key equal to `sentinel`
///   (typically the program ID).
///
/// # Construction
///
/// Use [`AccountVec::parse`] inside your instruction's `build()` method,
/// passing the slice starting immediately after the fixed accounts:
///
/// ```ignore
/// const FIXED: usize = 9; // number of fixed accounts
///
/// let groups = AccountVec::<TraderGroup>::parse(&accounts[FIXED..], program_id)?;
/// ```
///
/// # Capacity
///
/// Groups are stored in a heap-allocated `Vec`.  On Solana the number of
/// accounts per transaction is bounded (typically 64), so the vec will never
/// hold more than `(64 - FIXED) / ACCOUNTS_PER_GROUP` elements.
pub struct AccountVec<'a, T: FromAccountSlice<'a>> {
    /// Parsed groups, in the order they appeared in the account slice.
    pub groups: Vec<T>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, T: FromAccountSlice<'a>> AccountVec<'a, T> {
    /// Parse groups from `accounts`, stopping at `sentinel` or end-of-slice.
    ///
    /// # Arguments
    ///
    /// * `accounts` - Tail slice beginning immediately after the fixed accounts.
    ///   Must already be offset past any fixed accounts.
    /// * `sentinel` - Address that signals end-of-list (typically the program ID).
    ///   When the first account in the remaining slice matches this address, parsing
    ///   stops and the sentinel is **not** consumed from the slice.
    ///
    /// # Errors
    ///
    /// Propagates any `Err` returned by [`FromAccountSlice::from_slice`] for
    /// individual group validation failures.  A partial trailing group (fewer
    /// remaining accounts than `ACCOUNTS_PER_GROUP`, with no sentinel) is
    /// treated as end-of-list rather than an error, making the sentinel truly
    /// optional when the vec is last in the account list.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Fixed accounts are at indices 0..9; variable groups start at index 9.
    /// let vec = AccountVec::<TraderGroup>::parse(&accounts[9..], program_id)?;
    /// pinocchio_log::log!("settling {} traders", vec.len());
    /// ```
    pub fn parse(accounts: &'a [AccountView], sentinel: &Address) -> Result<Self, ProgramError> {
        let mut groups = Vec::new();
        let mut remaining = accounts;

        loop {
            // Stop at end of slice.
            if remaining.is_empty() {
                break;
            }

            // Stop at sentinel: the first account's key equals the program ID (or
            // whatever address the caller designates as the boundary marker).
            if address_eq(remaining[0].address(), sentinel) {
                break;
            }

            // We have the start of a new group.  If fewer than a full group's worth
            // of accounts remain (and no sentinel was found), the vec is simply
            // exhausted — treat it the same as end-of-slice.  This makes the
            // sentinel truly optional when the vec is the last set of accounts.
            if remaining.len() < T::ACCOUNTS_PER_GROUP {
                break;
            }

            // Parse exactly ACCOUNTS_PER_GROUP accounts as one group.
            let group = T::from_slice(&remaining[..T::ACCOUNTS_PER_GROUP])?;
            groups.push(group);

            // Advance past the consumed accounts.
            remaining = &remaining[T::ACCOUNTS_PER_GROUP..];
        }

        Ok(Self { groups, _phantom: PhantomData })
    }

    /// Iterate over the parsed groups (immutable).
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.groups.iter()
    }

    /// Iterate over the parsed groups (mutable).
    #[inline]
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.groups.iter_mut()
    }

    /// Number of parsed groups.
    #[inline]
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Returns `true` if no groups were parsed.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}
