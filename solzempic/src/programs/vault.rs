//! Vault account wrappers.
//!
//! This module provides wrappers for vault accounts (program-controlled
//! token accounts and SOL accounts):
//!
//! - [`Vault`] - SPL Token vault with authority validation
//! - [`SolVault`] - Native SOL vault (system-owned account)

use pinocchio::{error::ProgramError, AccountView};
use solana_address::{Address, address_eq};

use super::ids::{SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};

/// Vault token account with authority validation.
///
/// `Vault` wraps a token account that serves as a program-controlled vault,
/// validating both token program ownership and the token account's authority.
/// This ensures the vault is controlled by the expected PDA or authority.
///
/// # Validation
///
/// On wrap, the following checks are performed:
/// 1. Account is owned by Token or Token-2022 program
/// 2. Token account's `owner` field matches the expected authority
///
/// # Example
///
/// ```ignore
/// use solzempic::Vault;
///
/// fn deposit<'a>(accounts: &'a [AccountInfo], program_id: &Pubkey) -> ProgramResult {
///     // Market PDA controls the vault
///     let (market_pda, _) = Pubkey::find_program_address(
///         &[b"market", base_mint.as_ref()],
///         program_id,
///     );
///
///     // Validate vault is controlled by market PDA
///     let vault = Vault::wrap(&accounts[0], &market_pda)?;
///
///     let vault_balance = vault.amount();
///     Ok(())
/// }
/// ```
///
/// # When to Use
///
/// Use `Vault` for token accounts that your program controls:
/// - DEX base/quote vaults
/// - Lending protocol reserves
/// - Staking reward pools
/// - Any PDA-owned token account
pub struct Vault<'a> {
    info: &'a AccountView,
}

impl<'a> Vault<'a> {
    /// Wrap and validate a vault token account.
    ///
    /// # Arguments
    ///
    /// * `info` - The vault token account
    /// * `expected_authority` - Expected owner of the token account (usually a PDA)
    ///
    /// # Errors
    ///
    /// * [`ProgramError::IllegalOwner`] - Not owned by a token program
    /// * [`ProgramError::InvalidAccountData`] - Too small or wrong authority
    #[inline]
    pub fn wrap(info: &'a AccountView, expected_authority: &Address) -> Result<Self, ProgramError> {
        let owner = unsafe { info.owner() };
        if !address_eq(owner, &TOKEN_PROGRAM_ID) && !address_eq(owner, &TOKEN_2022_PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }

        // Validate token account owner matches expected authority
        let data = unsafe { info.borrow_unchecked() };
        if data.len() < 64 {
            return Err(ProgramError::InvalidAccountData);
        }
        let token_owner: &Address = unsafe { &*(data[32..64].as_ptr() as *const Address) };
        if !address_eq(token_owner, expected_authority) {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self { info })
    }

    /// Get the underlying AccountView.
    #[inline]
    pub fn info(&self) -> &'a AccountView {
        self.info
    }

    /// Get the vault's address.
    #[inline]
    pub fn address(&self) -> &'a Address {
        self.info.address()
    }

    /// Get the token amount held in the vault.
    ///
    /// Reads the amount directly from the token account data at offset 64.
    #[inline]
    pub fn amount(&self) -> u64 {
        let data = unsafe { self.info.borrow_unchecked() };
        let bytes: [u8; 8] = data[64..72].try_into().unwrap_or([0; 8]);
        u64::from_le_bytes(bytes)
    }
}

/// SOL vault account wrapper.
///
/// `SolVault` wraps a system-owned account used to hold native SOL.
/// Unlike token vaults, SOL vaults are simply accounts owned by the
/// System program whose lamport balance represents the stored value.
///
/// # Use Cases
///
/// - Collecting SOL fees
/// - Holding native SOL reserves
/// - PDA-controlled SOL storage
///
/// # Example
///
/// ```ignore
/// use solzempic::SolVault;
///
/// fn collect_fee<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let fee_vault = SolVault::wrap(&accounts[0])?;
///
///     let balance = fee_vault.lamports();
///     msg!("Fee vault balance: {} lamports", balance);
///
///     Ok(())
/// }
/// ```
///
/// # Note
///
/// To transfer SOL from a `SolVault`, use [`transfer_lamports`](crate::transfer_lamports)
/// or direct lamport manipulation if the program owns the account via PDA.
pub struct SolVault<'a> {
    info: &'a AccountView,
}

impl<'a> SolVault<'a> {
    /// Wrap a SOL vault account.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::IllegalOwner`] if the account is not
    /// owned by the System program.
    #[inline]
    pub fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !address_eq(unsafe { info.owner() }, &SYSTEM_PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(Self { info })
    }

    /// Get the underlying AccountView.
    #[inline]
    pub fn info(&self) -> &'a AccountView {
        self.info
    }

    /// Get the vault's address.
    #[inline]
    pub fn address(&self) -> &'a Address {
        self.info.address()
    }

    /// Get the lamport (SOL) balance.
    ///
    /// 1 SOL = 1,000,000,000 lamports.
    #[inline]
    pub fn lamports(&self) -> u64 {
        self.info.lamports()
    }

    /// Check if this account is writable.
    ///
    /// The account must be writable to receive or send lamports.
    #[inline]
    pub fn is_writable(&self) -> bool {
        self.info.is_writable()
    }
}
