//! Program and sysvar account IDs.
//!
//! This module contains well-known program IDs and sysvar addresses used by
//! Solana programs. These are compile-time constants that can be used for
//! account validation without runtime lookups.
//!
//! # Programs
//!
//! | Constant | Program | Use Case |
//! |----------|---------|----------|
//! | [`SYSTEM_PROGRAM_ID`] | Native System | Account creation, transfers |
//! | [`TOKEN_PROGRAM_ID`] | SPL Token | Token operations |
//! | [`TOKEN_2022_PROGRAM_ID`] | Token-2022 | Extended token features |
//! | [`ASSOCIATED_TOKEN_PROGRAM_ID`] | ATA Program | Derive token accounts |
//! | [`ADDRESS_LOOKUP_TABLE_PROGRAM_ID`] | ALT Program | Transaction compression |
//!
//! # Sysvars
//!
//! | Constant | Sysvar | Data |
//! |----------|--------|------|
//! | [`CLOCK_SYSVAR_ID`] | Clock | Current slot, timestamp, epoch |
//! | [`RENT_SYSVAR_ID`] | Rent | Rent calculation parameters |
//! | [`SLOT_HASHES_SYSVAR_ID`] | SlotHashes | Recent slot hashes |
//! | [`INSTRUCTIONS_SYSVAR_ID`] | Instructions | Transaction introspection |
//! | [`RECENT_BLOCKHASHES_SYSVAR_ID`] | RecentBlockhashes | Recent blockhashes |
//!
//! # Example
//!
//! ```ignore
//! use solzempic::{SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID};
//!
//! // Validate account is system program
//! if account.key() != &SYSTEM_PROGRAM_ID {
//!     return Err(ProgramError::IncorrectProgramId);
//! }
//!
//! // Check if token account is owned by either token program
//! let owner = account.owner();
//! let is_token = owner == &TOKEN_PROGRAM_ID || owner == &TOKEN_2022_PROGRAM_ID;
//! ```

use solana_address::Address;

// ============================================================================
// Program IDs
// ============================================================================

/// The Solana System Program ID.
///
/// The System program is responsible for:
/// - Creating new accounts
/// - Allocating account data
/// - Assigning account ownership
/// - Transferring lamports between system-owned accounts
///
/// All newly created accounts are initially owned by the System program until
/// ownership is transferred to another program.
///
/// Address: `11111111111111111111111111111111`
pub const SYSTEM_PROGRAM_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("11111111111111111111111111111111"));

/// The SPL Token Program ID.
///
/// The original SPL Token program handles:
/// - Token mints (creating fungible tokens)
/// - Token accounts (holding tokens)
/// - Transfers, approvals, burns, and minting
///
/// For newer features like transfer hooks and confidential transfers,
/// use [`TOKEN_2022_PROGRAM_ID`] instead.
///
/// Address: `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
pub const TOKEN_PROGRAM_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"));

/// The Token-2022 (Token Extensions) Program ID.
///
/// Token-2022 extends SPL Token with additional features:
/// - Transfer hooks (execute custom logic on transfers)
/// - Confidential transfers (encrypted amounts)
/// - Interest-bearing tokens
/// - Non-transferable tokens
/// - Permanent delegate
/// - Transfer fees
///
/// Address: `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`
pub const TOKEN_2022_PROGRAM_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"));

/// The Associated Token Account (ATA) Program ID.
///
/// The ATA program provides deterministic token account addresses:
/// - Derives token accounts from (wallet, mint) pairs
/// - Creates accounts idempotently
/// - Simplifies token account management
///
/// ATA addresses are derived as:
/// ```ignore
/// seeds = [wallet, TOKEN_PROGRAM_ID, mint]
/// PDA = find_program_address(seeds, ASSOCIATED_TOKEN_PROGRAM_ID)
/// ```
///
/// Address: `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL`
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"));

/// The Address Lookup Table (ALT) Program ID.
///
/// The ALT program enables transaction compression by:
/// - Storing frequently-used addresses in lookup tables
/// - Referencing addresses by 1-byte index instead of 32-byte pubkey
/// - Reducing transaction size for complex multi-account operations
///
/// Lookup tables are particularly useful for:
/// - DEX transactions with many token accounts
/// - NFT batch operations
/// - Cross-program invocations
///
/// Address: `AddressLookupTab1e1111111111111111111111111`
pub const ADDRESS_LOOKUP_TABLE_PROGRAM_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("AddressLookupTab1e1111111111111111111111111"));

// ============================================================================
// Sysvar IDs
// ============================================================================

/// The Clock sysvar address.
///
/// The Clock sysvar provides timing information:
/// - `slot`: Current slot number
/// - `epoch_start_timestamp`: Unix timestamp of epoch start
/// - `epoch`: Current epoch number
/// - `leader_schedule_epoch`: Epoch for which leader schedule is valid
/// - `unix_timestamp`: Estimated current Unix timestamp
///
/// # Usage
///
/// ```ignore
/// let clock = Clock::from_account_info(&clock_account)?;
/// let current_time = clock.unix_timestamp;
/// ```
///
/// Address: `SysvarC1ock11111111111111111111111111111111`
pub const CLOCK_SYSVAR_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("SysvarC1ock11111111111111111111111111111111"));

/// The Rent sysvar address.
///
/// The Rent sysvar provides rent parameters for account storage:
/// - `lamports_per_byte_year`: Lamports charged per byte per year
/// - `exemption_threshold`: Multiplier for rent exemption calculation
/// - `burn_percent`: Percentage of rent collected that is burned
///
/// Use this to calculate the minimum balance for rent exemption.
///
/// Address: `SysvarRent111111111111111111111111111111111`
pub const RENT_SYSVAR_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("SysvarRent111111111111111111111111111111111"));

/// The SlotHashes sysvar address.
///
/// The SlotHashes sysvar contains recent slot hashes:
/// - Up to 512 recent (slot, hash) pairs
/// - Useful for verifiable randomness
/// - Can verify that a slot occurred recently
///
/// # Caution
///
/// SlotHashes is large (~16KB). Avoid passing it unless necessary,
/// as it consumes significant transaction space.
///
/// Address: `SysvarS1otHashes111111111111111111111111111`
pub const SLOT_HASHES_SYSVAR_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("SysvarS1otHashes111111111111111111111111111"));

/// The Instructions sysvar address.
///
/// The Instructions sysvar enables transaction introspection:
/// - Access all instructions in the current transaction
/// - Verify instruction ordering
/// - Check for specific program calls
///
/// Useful for:
/// - Flash loan protection (ensure repayment instruction exists)
/// - Multi-instruction atomicity checks
/// - Signature verification
///
/// Address: `Sysvar1nstructions1111111111111111111111111`
pub const INSTRUCTIONS_SYSVAR_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("Sysvar1nstructions1111111111111111111111111"));

/// The RecentBlockhashes sysvar address (deprecated).
///
/// **Note**: This sysvar is deprecated. New programs should use the
/// Clock sysvar or SlotHashes sysvar instead.
///
/// Previously provided recent blockhashes for transaction verification
/// and nonce-based replay protection.
///
/// Address: `SysvarRecentB1ockHashes11111111111111111111`
pub const RECENT_BLOCKHASHES_SYSVAR_ID: Address =
    Address::new_from_array(pinocchio_pubkey::pubkey!("SysvarRecentB1ockHashes11111111111111111111"));
