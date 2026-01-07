//! Type-safe account wrappers that combine AccountInfo with parsed data.
//!
//! This module provides the core account wrapper types used throughout Solzempic.
//! These wrappers eliminate the need to manually manage account data parsing,
//! ownership validation, and mutability checks.
//!
//! # Wrapper Types
//!
//! | Type | Purpose | Validates |
//! |------|---------|-----------|
//! | [`AccountRef<T, F>`] | Read-only access | Owner, discriminator, size |
//! | [`AccountRefMut<T, F>`] | Writable access | Owner, discriminator, size, `is_writable` |
//! | [`ShardRefContext<T, F>`] | Shard triplet | All three accounts loaded as `AccountRefMut` |
//!
//! # The Framework Pattern
//!
//! All wrappers are generic over `F: Framework`. This allows ownership validation
//! to use your program's ID without passing it as a runtime parameter:
//!
//! ```ignore
//! // In lib.rs - define your framework once:
//! solzempic::define_framework!(MY_PROGRAM_ID);
//!
//! // Now AccountRef and AccountRefMut automatically validate against MY_PROGRAM_ID:
//! let account = AccountRef::<MyAccount, Solzempic>::load(&accounts[0])?;
//! //                                   ^^^^^^^^^ your framework type
//! ```
//!
//! The [`define_framework!`](crate::define_framework) macro creates convenient
//! type aliases so you can write `AccountRef<'a, MyAccount>` instead of the
//! fully-qualified form.
//!
//! # Usage Example
//!
//! ```ignore
//! use solzempic::{AccountRef, AccountRefMut, Signer};
//!
//! pub struct Transfer<'a> {
//!     pub from: AccountRefMut<'a, Wallet>,
//!     pub to: AccountRefMut<'a, Wallet>,
//!     pub owner: Signer<'a>,
//! }
//!
//! impl<'a> Transfer<'a> {
//!     pub fn build(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
//!         Ok(Self {
//!             from: AccountRefMut::load(&accounts[0])?,  // Validates owner + writable
//!             to: AccountRefMut::load(&accounts[1])?,    // Validates owner + writable
//!             owner: Signer::wrap(&accounts[2])?,        // Validates is_signer
//!         })
//!     }
//! }
//! ```
//!
//! # Performance
//!
//! All wrapper methods are `#[inline]` and compile to minimal runtime overhead:
//!
//! - Loading: Single borrow + ownership memcmp (~50 CUs)
//! - Data access: Zero-copy pointer cast (~5 CUs)
//! - PDA validation: Derive + compare (~2000 CUs, use sparingly)
//!
//! # Traits
//!
//! The [`AsAccountRef`] trait provides a common interface for both read-only
//! and writable wrappers, allowing generic code to work with either type.

mod account_ref;
mod account_ref_mut;
mod shard_ref_context;
mod traits;

pub use account_ref::AccountRef;
pub use account_ref_mut::AccountRefMut;
pub use shard_ref_context::ShardRefContext;
pub use traits::AsAccountRef;
