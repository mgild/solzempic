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

impl<'a> ClockSysvar<'a> {
    /// Get the Clock struct from this sysvar.
    #[inline]
    pub fn get(&self) -> Result<pinocchio::account::Ref<'a, pinocchio::sysvars::clock::Clock>, ProgramError> {
        pinocchio::sysvars::clock::Clock::from_account_view(self.info)
    }
}

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

define_sysvar!(
    LastRestartSlotSysvar,
    LAST_RESTART_SLOT_SYSVAR_ID,
    "Validated LastRestartSlot sysvar account.\n\nProvides the slot number of the last cluster restart (hard fork), or 0 if none.\nUseful for DeFi protocols to detect stale oracle prices after a cluster restart."
);

/// LastRestartSlot sysvar data structure.
///
/// Contains the slot number of the last restart (hard fork), or 0 if none ever happened.
/// This information helps DeFi protocols prevent arbitrage and liquidation caused by
/// outdated oracle price account states after a cluster restart.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LastRestartSlot {
    /// The slot number of the last restart, or 0 if none.
    pub last_restart_slot: u64,
}

impl<'a> LastRestartSlotSysvar<'a> {
    /// Get the LastRestartSlot data from this sysvar account.
    ///
    /// Reads the u64 value directly from account data.
    #[inline]
    pub fn get(&self) -> Result<LastRestartSlot, ProgramError> {
        let data = unsafe { self.info.borrow_unchecked() };
        if data.len() < 8 {
            return Err(ProgramError::InvalidAccountData);
        }
        let last_restart_slot = u64::from_le_bytes(data[0..8].try_into().unwrap());
        Ok(LastRestartSlot { last_restart_slot })
    }

    /// Get the last restart slot directly via syscall (no account needed).
    ///
    /// This is more efficient than passing the sysvar account.
    #[inline]
    pub fn get_via_syscall() -> Result<LastRestartSlot, ProgramError> {
        let mut var = core::mem::MaybeUninit::<LastRestartSlot>::uninit();
        let var_addr = var.as_mut_ptr() as *mut u8;

        #[cfg(target_os = "solana")]
        let result = unsafe { pinocchio::syscalls::sol_get_last_restart_slot(var_addr) };

        #[cfg(not(target_os = "solana"))]
        let result = {
            let _ = var_addr;
            0u64 // Return success for non-solana targets
        };

        match result {
            0 => Ok(unsafe { var.assume_init() }),
            _ => Err(ProgramError::UnsupportedSysvar),
        }
    }
}
