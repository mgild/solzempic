//! Sysvar account wrappers.
//!
//! This module provides validated wrappers for Solana sysvars. Sysvars are
//! special accounts that provide cluster state data to programs.
//!
//! # Available Sysvars
//!
//! | Wrapper | Provides |
//! |---------|----------|
//! | [`ClockSysvar`] | Current slot, timestamp, epoch |
//! | [`RentSysvar`] | Rent calculation parameters |
//! | [`SlotHashesSysvar`] | Recent slot hashes (large!) |
//! | [`InstructionsSysvar`] | Transaction introspection |
//! | [`RecentBlockhashesSysvar`] | Recent blockhashes (deprecated) |
//!
//! # Example
//!
//! ```ignore
//! use solzempic::{ValidatedAccount, ClockSysvar};
//!
//! fn check_expiration(accounts: &[AccountInfo]) -> ProgramResult {
//!     let clock = ClockSysvar::wrap(&accounts[0])?;
//!     let current_time = unsafe { *(clock.info().data_ptr() as *const i64) };
//!     // ... check order expiration
//!     Ok(())
//! }
//! ```
//!
//! # Direct Sysvar Access
//!
//! For sysvars that support it, you can also use `solana_program`'s
//! `Sysvar::get()` method which doesn't require an account. However,
//! this uses more compute units.

use pinocchio::{AccountView, error::ProgramError};
use solana_address::address_eq;

use super::ids::*;
use super::traits::ValidatedAccount;

/// Define a validated sysvar wrapper with documentation.
macro_rules! define_sysvar {
    ($name:ident, $id:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// This wrapper validates that the account key matches the expected
        /// sysvar address. After validation, you can access the sysvar data
        /// through `info().borrow_data_unchecked()`.
        ///
        /// # Performance
        ///
        /// Validation cost: ~20 CUs (single 32-byte key comparison)
        pub struct $name<'a> {
            info: &'a AccountView,
        }

        impl<'a> ValidatedAccount<'a> for $name<'a> {
            /// Validate that the account is this sysvar.
            ///
            /// # Errors
            ///
            /// Returns [`ProgramError::IncorrectProgramId`] if the account key
            /// does not match the expected sysvar address.
            #[inline]
            fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
                if !address_eq(info.address(), &$id) {
                    return Err(ProgramError::IncorrectProgramId);
                }
                Ok(Self { info })
            }

            #[inline]
            fn info(&self) -> &'a AccountView {
                self.info
            }
        }
    };
}

define_sysvar!(
    ClockSysvar,
    CLOCK_SYSVAR_ID,
    "Validated Clock sysvar account.\n\nProvides current slot, epoch, and Unix timestamp.\nUse for time-based logic like order expiration."
);

define_sysvar!(
    RentSysvar,
    RENT_SYSVAR_ID,
    "Validated Rent sysvar account.\n\nProvides rent calculation parameters.\nUseful for computing rent-exempt minimum balances."
);

define_sysvar!(
    SlotHashesSysvar,
    SLOT_HASHES_SYSVAR_ID,
    "Validated SlotHashes sysvar account.\n\nProvides recent slot hashes for randomness or verification.\n\n**Warning**: This sysvar is large (~16KB). Avoid unless necessary."
);

define_sysvar!(
    InstructionsSysvar,
    INSTRUCTIONS_SYSVAR_ID,
    "Validated Instructions sysvar account.\n\nEnables transaction introspection.\nUseful for flash loan protection and multi-instruction checks."
);

define_sysvar!(
    RecentBlockhashesSysvar,
    RECENT_BLOCKHASHES_SYSVAR_ID,
    "Validated RecentBlockhashes sysvar account.\n\n**Deprecated**: Use Clock or SlotHashes instead.\nPreviously provided recent blockhashes for nonce-based replay protection."
);
