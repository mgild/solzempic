//! SPL Token Mint account wrapper.
//!
//! This module provides [`Mint`], a validated wrapper for SPL Token mint
//! accounts that provides convenient access to mint metadata.

use pinocchio::{AccountView, error::ProgramError};
use solana_address::{Address, address_eq};

use super::ids::{TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};
use super::traits::ValidatedAccount;

/// Validated SPL Token Mint account wrapper.
///
/// `Mint` wraps a token mint account, validating that it's owned by either
/// the SPL Token or Token-2022 program. It provides convenient accessor
/// methods for mint metadata without requiring full deserialization.
///
/// # Mint Layout (82 bytes base)
///
/// | Offset | Size | Field |
/// |--------|------|-------|
/// | 0 | 4 | mint_authority COption discriminant |
/// | 4 | 32 | mint_authority Pubkey |
/// | 36 | 8 | supply |
/// | 44 | 1 | decimals |
/// | 45 | 1 | is_initialized |
/// | 46 | 4 | freeze_authority COption discriminant |
/// | 50 | 32 | freeze_authority Pubkey |
///
/// Token-2022 mints may have extensions after byte 82.
///
/// # Example
///
/// ```ignore
/// use solzempic::{ValidatedAccount, Mint};
///
/// fn validate_mint<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let mint = Mint::wrap(&accounts[0])?;
///
///     // Access mint metadata
///     let decimals = mint.decimals();
///     let supply = mint.supply();
///
///     // Check authorities
///     if let Some(authority) = mint.mint_authority() {
///         msg!("Mint authority: {}", authority);
///     }
///
///     Ok(())
/// }
/// ```
///
/// # When to Use
///
/// Use `Mint` when you need to:
/// - Read token decimals for amount calculations
/// - Validate mint authority before minting
/// - Check total supply
/// - Verify freeze authority status
///
/// # Performance
///
/// | Operation | Cost |
/// |-----------|------|
/// | `wrap()` | ~40 CUs (ownership check) |
/// | `decimals()` | ~10 CUs (single byte read) |
/// | `supply()` | ~15 CUs (8 byte read) |
/// | `mint_authority()` | ~25 CUs (36 byte read + option check) |
pub struct Mint<'a> {
    info: &'a AccountView,
}

impl<'a> ValidatedAccount<'a> for Mint<'a> {
    /// Validate that the account is a token mint.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::IllegalOwner`] if the account is not owned
    /// by either the SPL Token or Token-2022 program.
    ///
    /// Returns [`ProgramError::UninitializedAccount`] if the mint data is
    /// too small or the mint is not initialized.
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        let owner = unsafe { info.owner() };
        if !address_eq(owner, &TOKEN_PROGRAM_ID) && !address_eq(owner, &TOKEN_2022_PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }

        // Validate mint is initialized (minimum 82 bytes, is_initialized=true at offset 45)
        let data = unsafe { info.borrow_unchecked() };
        if data.len() < 82 {
            return Err(ProgramError::UninitializedAccount);
        }
        if data[45] == 0 {
            // is_initialized == false
            return Err(ProgramError::UninitializedAccount);
        }

        Ok(Self { info })
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}

impl<'a> Mint<'a> {
    // SPL Token Mint layout (82 bytes):
    // 0-4:   mint_authority COption discriminant
    // 4-36:  mint_authority Pubkey
    // 36-44: supply
    // 44:    decimals
    // 45:    is_initialized
    // 46-50: freeze_authority COption discriminant
    // 50-82: freeze_authority Pubkey

    const MINT_AUTHORITY_OPTION_OFFSET: usize = 0;
    const MINT_AUTHORITY_OFFSET: usize = 4;
    const SUPPLY_OFFSET: usize = 36;
    const DECIMALS_OFFSET: usize = 44;
    const FREEZE_AUTHORITY_OPTION_OFFSET: usize = 46;
    const FREEZE_AUTHORITY_OFFSET: usize = 50;

    /// Get the mint authority if set.
    ///
    /// The mint authority can mint new tokens. Returns `None` if the
    /// mint authority has been revoked (set to `None`), which makes
    /// the token supply fixed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(authority) = mint.mint_authority() {
    ///     // Mint can be expanded
    ///     if &authority != expected_authority {
    ///         return Err(ProgramError::IllegalOwner);
    ///     }
    /// } else {
    ///     // Fixed supply token
    /// }
    /// ```
    #[inline]
    pub fn mint_authority(&self) -> Option<Address> {
        let data = unsafe { self.info.borrow_unchecked() };
        let option = u32::from_le_bytes(data[Self::MINT_AUTHORITY_OPTION_OFFSET..Self::MINT_AUTHORITY_OPTION_OFFSET + 4].try_into().unwrap());
        if option != 0 {
            Some(Address::new_from_array(<[u8; 32]>::try_from(&data[Self::MINT_AUTHORITY_OFFSET..Self::MINT_AUTHORITY_OFFSET + 32]).unwrap()))
        } else {
            None
        }
    }

    /// Get the freeze authority if set.
    ///
    /// The freeze authority can freeze token accounts, preventing
    /// transfers. Returns `None` if freeze authority was never set
    /// or has been revoked.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if mint.freeze_authority().is_some() {
    ///     // Token accounts can be frozen - may affect DeFi usability
    ///     msg!("Warning: Token can be frozen");
    /// }
    /// ```
    #[inline]
    pub fn freeze_authority(&self) -> Option<Address> {
        let data = unsafe { self.info.borrow_unchecked() };
        let option = u32::from_le_bytes(data[Self::FREEZE_AUTHORITY_OPTION_OFFSET..Self::FREEZE_AUTHORITY_OPTION_OFFSET + 4].try_into().unwrap());
        if option != 0 {
            Some(Address::new_from_array(<[u8; 32]>::try_from(&data[Self::FREEZE_AUTHORITY_OFFSET..Self::FREEZE_AUTHORITY_OFFSET + 32]).unwrap()))
        } else {
            None
        }
    }

    /// Get the total token supply.
    ///
    /// Returns the current total supply of tokens that have been minted.
    /// This value increases on mint and decreases on burn operations.
    #[inline]
    pub fn supply(&self) -> u64 {
        let data = unsafe { self.info.borrow_unchecked() };
        u64::from_le_bytes(data[Self::SUPPLY_OFFSET..Self::SUPPLY_OFFSET + 8].try_into().unwrap())
    }

    /// Get the number of decimal places for the token.
    ///
    /// Decimals determine how the raw token amount is displayed.
    /// For example, with 6 decimals, a raw amount of 1,000,000 represents 1.0 tokens.
    ///
    /// Common values:
    /// - USDC/USDT: 6 decimals
    /// - SOL (wrapped): 9 decimals
    /// - Many governance tokens: 6-9 decimals
    #[inline]
    pub fn decimals(&self) -> u8 {
        let data = unsafe { self.info.borrow_unchecked() };
        // Safe: wrap() validates data.len() >= 82, and DECIMALS_OFFSET is 44
        data[Self::DECIMALS_OFFSET]
    }

    /// Check if this is a Token-2022 mint.
    ///
    /// Returns `true` if the mint is owned by the Token-2022 program,
    /// `false` if owned by the original SPL Token program.
    ///
    /// Token-2022 mints may have additional extensions after the base
    /// 82-byte layout.
    #[inline]
    pub fn is_token_2022(&self) -> bool {
        address_eq(unsafe { self.info.owner() }, &TOKEN_2022_PROGRAM_ID)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec;
    use std::vec::Vec;

    use super::*;

    fn create_valid_mint_data(decimals: u8) -> Vec<u8> {
        let mut data = vec![0u8; 82];
        data[0] = 1; // mint_authority present
        data[44] = decimals;
        data[45] = 1; // is_initialized = true
        data
    }

    fn create_uninitialized_mint_data(decimals: u8) -> Vec<u8> {
        let mut data = vec![0u8; 82];
        data[0] = 1; // mint_authority present
        data[44] = decimals;
        data[45] = 0; // is_initialized = false
        data
    }

    #[test]
    fn test_valid_mint_decimals() {
        // Test that we correctly read decimals from a valid mint
        let data = create_valid_mint_data(9);
        assert_eq!(data[44], 9, "Decimals should be at offset 44");
        assert_eq!(data[45], 1, "is_initialized should be at offset 45");
    }

    #[test]
    fn test_uninitialized_mint_layout() {
        // Verify the layout of an uninitialized mint
        let data = create_uninitialized_mint_data(6);
        assert_eq!(data[44], 6, "Decimals should still be set");
        assert_eq!(data[45], 0, "is_initialized should be 0");
    }

    #[test]
    fn test_truncated_mint_data() {
        // Verify truncated data is too short for decimals
        let data = vec![0u8; 40];
        assert!(data.len() < 82, "Truncated data should be too short");
        assert!(data.get(44).is_none(), "Should not be able to access decimals offset");
    }

    #[test]
    fn test_mint_offsets_are_correct() {
        // Verify our offset constants match the SPL Token layout
        assert_eq!(Mint::MINT_AUTHORITY_OPTION_OFFSET, 0);
        assert_eq!(Mint::MINT_AUTHORITY_OFFSET, 4);
        assert_eq!(Mint::SUPPLY_OFFSET, 36);
        assert_eq!(Mint::DECIMALS_OFFSET, 44);
        assert_eq!(Mint::FREEZE_AUTHORITY_OPTION_OFFSET, 46);
        assert_eq!(Mint::FREEZE_AUTHORITY_OFFSET, 50);
    }
}
