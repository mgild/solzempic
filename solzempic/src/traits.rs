//! Core traits for account types.
//!
//! This module provides the foundational traits for working with Solana account data
//! in a type-safe, zero-copy manner.

use bytemuck::Pod;
use pinocchio::error::ProgramError;

/// Trait for account structs with discriminator field access.
///
/// This trait provides common discriminator handling and validation methods
/// for account types that have an 8-byte discriminator field.
///
/// # Example
///
/// ```ignore
/// impl Account for MyAccount {
///     const DISCRIMINATOR: u8 = 1;
///     const LEN: usize = core::mem::size_of::<Self>();
///
///     fn discriminator(&self) -> &[u8; 8] {
///         &self.discriminator
///     }
/// }
/// ```
pub trait Account: Pod {
    /// The discriminator byte for this account type.
    const DISCRIMINATOR: u8;

    /// Account size in bytes.
    const LEN: usize;

    /// Get the discriminator bytes from the account data.
    fn discriminator(&self) -> &[u8; 8];

    /// Verify this account has the correct discriminator.
    #[inline]
    fn verify_discriminator(&self) -> bool {
        self.discriminator()[0] == Self::DISCRIMINATOR
    }

    /// Check raw account data without parsing.
    #[inline]
    fn check_data(data: &[u8]) -> bool {
        check_discriminator(data, Self::DISCRIMINATOR)
    }

    /// Load account from raw data, validating discriminator.
    #[inline]
    fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if !Self::check_data(data) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(bytemuck::from_bytes(&data[..Self::LEN]))
    }

    /// Load mutable account from raw data, validating discriminator.
    #[inline]
    fn load_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if !Self::check_data(data) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(bytemuck::from_bytes_mut(&mut data[..Self::LEN]))
    }

    /// Load account from raw data without discriminator check (unchecked).
    #[inline]
    fn load_unchecked(data: &[u8]) -> &Self {
        bytemuck::from_bytes(&data[..Self::LEN])
    }

    /// Load mutable account from raw data without discriminator check (unchecked).
    #[inline]
    fn load_unchecked_mut(data: &mut [u8]) -> &mut Self {
        bytemuck::from_bytes_mut(&mut data[..Self::LEN])
    }
}

/// Trait for Pod types that can be loaded from account data.
///
/// Implement this for `bytemuck::Pod` structs that represent account data.
/// The discriminator is a single byte that identifies the account type.
///
/// # Example
///
/// ```ignore
/// use bytemuck::{Pod, Zeroable};
/// use solzempic::Loadable;
///
/// #[repr(C)]
/// #[derive(Clone, Copy, Pod, Zeroable)]
/// pub struct Counter {
///     pub discriminator: u8,
///     pub _padding: [u8; 7],
///     pub owner: [u8; 32],
///     pub count: u64,
/// }
///
/// impl Loadable for Counter {
///     const DISCRIMINATOR: u8 = 1; // Or use AccountType::Counter as u8
///     const LEN: usize = core::mem::size_of::<Self>();
/// }
/// ```
pub trait Loadable: Pod + Sized {
    /// The discriminator byte for this account type.
    ///
    /// This is checked when loading accounts to ensure the data
    /// matches the expected type.
    const DISCRIMINATOR: u8;

    /// Size of the type in bytes.
    ///
    /// Defaults to `size_of::<Self>()` but can be overridden if needed.
    const LEN: usize = core::mem::size_of::<Self>();
}

/// Marker trait for types that can be initialized.
///
/// Types implementing this trait can be initialized via `AccountRefMut::init()`.
/// Initialization writes the discriminator byte and zeros the rest of the account.
///
/// # Example
///
/// ```ignore
/// use solzempic::{Loadable, Initializable};
///
/// #[repr(C)]
/// #[derive(Clone, Copy, Pod, Zeroable)]
/// pub struct Counter {
///     pub discriminator: u8,
///     pub _padding: [u8; 7],
///     pub owner: [u8; 32],
///     pub count: u64,
/// }
///
/// impl Loadable for Counter {
///     const DISCRIMINATOR: u8 = 1;
/// }
///
/// impl Initializable for Counter {}
/// ```
///
/// After calling `AccountRefMut::init()`, the account will have the discriminator
/// set and all other fields zeroed. You can then set fields via `get_mut()`.
pub trait Initializable: Loadable {}

/// Check if account data has the expected discriminator.
///
/// This is a helper function used by [`AccountRef`](crate::AccountRef) and
/// [`AccountRefMut`](crate::AccountRefMut) to validate account types.
///
/// # Arguments
///
/// * `data` - The raw account data bytes
/// * `expected` - The expected discriminator byte
///
/// # Returns
///
/// `true` if the data is non-empty and the first byte matches the expected discriminator.
#[inline]
pub fn check_discriminator(data: &[u8], expected: u8) -> bool {
    !data.is_empty() && data[0] == expected
}

// ============================================================================
// PDA Traits
// ============================================================================

/// Trait for PDA (Program Derived Address) types.
///
/// Each PDA type stores its derivation seeds and can provide them
/// for account creation via `init_pda`.
///
/// The associated type `AccountType` carries account type information
/// for type-safe PDA initialization (use `()` if not needed).
pub trait Pda {
    /// The account type this PDA is for (use `()` if not type-specific)
    type AccountType;

    /// The seeds array type (e.g., `[Seed; 4]`)
    type Seeds<'a>: AsRef<[pinocchio::cpi::Seed<'a>]>
    where
        Self: 'a;

    /// Get the seeds for this PDA, including the bump seed.
    fn seeds(&self) -> Self::Seeds<'_>;

    /// Get the bump seed as a single byte.
    fn bump(&self) -> u8;
}

/// Trait for PDA seed data types.
///
/// Implementors define how to build the seeds array from their fields
/// plus the bump seed. Used with `PdaBase<S>` for generic PDA construction.
pub trait PdaSeeds {
    type Seeds<'a>: AsRef<[pinocchio::cpi::Seed<'a>]>
    where
        Self: 'a;

    fn seeds<'a>(&'a self, bump: &'a [u8; 1]) -> Self::Seeds<'a>;
}

/// Generic PDA struct storing address, bump, and seed data.
///
/// Provides a blanket `Pda` impl for any `S: PdaSeeds`, eliminating
/// per-type boilerplate for `seeds()` and `bump()`.
///
/// The optional type parameter `T` can be used to carry account type information
/// for type-safe PDA initialization.
pub struct PdaBase<S, T = ()> {
    pub address: solana_address::Address,
    bump: [u8; 1],
    inner: S,
    _phantom: core::marker::PhantomData<T>,
}

impl<S, T> PdaBase<S, T> {
    #[inline]
    pub fn new(address: solana_address::Address, bump: u8, inner: S) -> Self {
        Self {
            address,
            bump: [bump],
            inner,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<S: PdaSeeds, T> Pda for PdaBase<S, T> {
    type AccountType = T;

    type Seeds<'a>
        = S::Seeds<'a>
    where
        Self: 'a;

    #[inline]
    fn seeds(&self) -> Self::Seeds<'_> {
        self.inner.seeds(&self.bump)
    }

    #[inline]
    fn bump(&self) -> u8 {
        self.bump[0]
    }
}

/// Trait for account types that can construct a PDA from a tuple.
///
/// This enables type-driven PDA construction where the account type
/// determines how to convert a tuple into the appropriate PDA.
pub trait FromTuplePda {
    /// The PDA type that can be constructed from a tuple.
    type Pda;

    /// Construct a PDA from a tuple (Address, u64).
    fn from_tuple(tuple: (pinocchio::Address, u64)) -> Self::Pda;
}
