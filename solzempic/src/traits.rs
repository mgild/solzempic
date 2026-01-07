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

/// Trait for types that can be initialized with params.
///
/// Implement this for account types that need initialization logic
/// beyond just zeroing the memory.
///
/// # Example
///
/// ```ignore
/// use solzempic::{Loadable, Initializable};
///
/// pub struct CounterParams {
///     pub owner: [u8; 32],
///     pub initial_count: u64,
/// }
///
/// impl Initializable for Counter {
///     type InitParams = CounterParams;
///
///     fn init(data: &mut [u8], params: Self::InitParams) -> Result<(), ProgramError> {
///         if data.len() < Self::LEN {
///             return Err(ProgramError::InvalidAccountData);
///         }
///
///         let counter: &mut Counter = bytemuck::from_bytes_mut(&mut data[..Self::LEN]);
///         counter.discriminator = Self::DISCRIMINATOR;
///         counter.owner = params.owner;
///         counter.count = params.initial_count;
///
///         Ok(())
///     }
/// }
/// ```
pub trait Initializable: Loadable {
    /// Parameters needed to initialize this account type.
    type InitParams;

    /// Initialize account data with the given params.
    ///
    /// This should:
    /// 1. Validate the data slice is large enough
    /// 2. Write the discriminator byte
    /// 3. Initialize all fields from params
    fn init(data: &mut [u8], params: Self::InitParams) -> Result<(), ProgramError>;
}

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
