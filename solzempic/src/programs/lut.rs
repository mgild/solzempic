//! Address Lookup Table account wrapper.
//!
//! This module provides [`Lut`], a wrapper for Address Lookup Table accounts
//! that handles both initialized and uninitialized states.

use pinocchio::{AccountView, error::ProgramError};
use solana_address::{Address, address_eq};

use super::ids::{ADDRESS_LOOKUP_TABLE_PROGRAM_ID, SYSTEM_PROGRAM_ID};

/// Address Lookup Table account wrapper.
///
/// `Lut` wraps a lookup table account, handling both initialized (active)
/// and uninitialized (not yet created) states. This makes it easy to
/// implement idempotent LUT creation patterns.
///
/// # Account States
///
/// | Owner | Discriminator | State |
/// |-------|---------------|-------|
/// | System Program | N/A | Uninitialized (needs creation) |
/// | ALT Program | 0 | Uninitialized (allocated but not set up) |
/// | ALT Program | 1 | Initialized (active lookup table) |
///
/// # Example
///
/// ```ignore
/// use solzempic::Lut;
///
/// fn ensure_lut_exists<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let lut = Lut::wrap(&accounts[0])?;
///
///     if lut.needs_init() {
///         // Create the lookup table via CPI
///         create_lookup_table(...)?;
///     } else {
///         // LUT already exists, can extend or use it
///     }
///
///     Ok(())
/// }
/// ```
///
/// # When to Use
///
/// Use `Lut` for:
/// - Checking if a LUT needs to be created
/// - Idempotent LUT initialization patterns
/// - Working with LUT accounts in CPI
///
/// # See Also
///
/// - [`AltProgram`](super::AltProgram) - The ALT program itself
pub struct Lut<'a> {
    info: &'a AccountView,
    initialized: bool,
}

impl<'a> Lut<'a> {
    /// Wrap a LUT account, determining its initialization state.
    ///
    /// Accepts both:
    /// - System-owned accounts (not yet created)
    /// - ALT-owned accounts (created but possibly not initialized)
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::IllegalOwner`] if the account is owned by
    /// neither the System program nor the ALT program.
    #[inline]
    pub fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        let owner = unsafe { info.owner() };

        // System-owned = not created yet
        if address_eq(owner, &SYSTEM_PROGRAM_ID) {
            return Ok(Self { info, initialized: false });
        }

        // ALT program owned
        if address_eq(owner, &ADDRESS_LOOKUP_TABLE_PROGRAM_ID) {
            let data = unsafe { info.borrow_unchecked() };
            // LUT type discriminator: 1 = LookupTable, 0 = Uninitialized
            let initialized = !data.is_empty() && data[0] == 1;
            return Ok(Self { info, initialized });
        }

        Err(ProgramError::IllegalOwner)
    }

    /// Get the underlying AccountView.
    #[inline]
    pub fn info(&self) -> &'a AccountView {
        self.info
    }

    /// Get the lookup table's address.
    #[inline]
    pub fn address(&self) -> &'a Address {
        self.info.address()
    }

    /// Check if the LUT is already initialized and active.
    ///
    /// An initialized LUT can be used in versioned transactions
    /// and can have addresses added to it.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if the LUT needs to be created or initialized.
    ///
    /// Returns `true` if the LUT should be created via the ALT program
    /// before it can be used.
    #[inline]
    pub fn needs_init(&self) -> bool {
        !self.initialized
    }
}
