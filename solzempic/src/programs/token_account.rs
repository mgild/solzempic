//! SPL Token Account wrappers.
//!
//! This module provides types for working with SPL Token accounts:
//!
//! - [`TokenAccountData`] - Zero-copy struct for token account layout
//! - [`TokenAccountRefMut`] - Writable wrapper with utility methods
//!
//! Both types work with SPL Token and Token-2022 accounts.

use pinocchio::{AccountView, error::ProgramError};
use solana_address::{Address, address_eq};

use super::ids::{ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};
use super::traits::HasAccountView;

/// SPL Token account data layout.
///
/// This struct provides zero-copy access to token account fields. It works
/// with both SPL Token and Token-2022 accounts (base fields are identical).
///
/// # Layout (165 bytes)
///
/// | Offset | Size | Field |
/// |--------|------|-------|
/// | 0 | 32 | mint |
/// | 32 | 32 | owner |
/// | 64 | 8 | amount |
/// | 72 | 4 | delegate COption discriminant |
/// | 76 | 32 | delegate |
/// | 108 | 1 | state |
/// | 109 | 4 | is_native COption discriminant |
/// | 113 | 8 | is_native value |
/// | 121 | 8 | delegated_amount |
/// | 129 | 4 | close_authority COption discriminant |
/// | 133 | 32 | close_authority |
///
/// Token-2022 accounts may have extensions after byte 165.
///
/// # Example
///
/// ```ignore
/// use solzempic::TokenAccountData;
///
/// // Cast from raw bytes
/// let data: &TokenAccountData = bytemuck::from_bytes(&account_data[..165]);
///
/// // Access fields
/// let amount = data.amount();
/// let owner = &data.owner;
/// ```
///
/// # Pod Safety
///
/// This struct is `#[repr(C)]` with alignment 1 (all fields are byte arrays
/// or `[u8; 32]` Pubkeys), so it's safe for zero-copy access via bytemuck.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TokenAccountData {
    /// The mint this account holds tokens for.
    pub mint: Address,
    /// The owner of this token account (can transfer tokens).
    pub owner: Address,
    amount: [u8; 8],
    delegate_tag: [u8; 4],
    /// The delegate authorized to transfer tokens (if set).
    pub delegate: Address,
    /// Account state: 0=Uninitialized, 1=Initialized, 2=Frozen.
    pub state: u8,
    is_native_tag: [u8; 4],
    is_native_val: [u8; 8],
    delegated_amount: [u8; 8],
    close_authority_tag: [u8; 4],
    /// The close authority (if set), authorized to close this account.
    pub close_authority: Address,
}

// Safety: Address is [u8; 32], all other fields are byte arrays - alignment 1, no padding
unsafe impl bytemuck::Pod for TokenAccountData {}
unsafe impl bytemuck::Zeroable for TokenAccountData {}

const _: () = assert!(core::mem::size_of::<TokenAccountData>() == 165);

impl TokenAccountData {
    /// Size of the token account data in bytes.
    pub const LEN: usize = 165;

    /// Get the token amount held in this account.
    #[inline]
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    /// Check if this account has an active delegate.
    ///
    /// A delegate can transfer up to `delegated_amount()` tokens on
    /// behalf of the owner.
    #[inline]
    pub fn has_delegate(&self) -> bool {
        u32::from_le_bytes(self.delegate_tag) != 0
    }

    /// Check if this is a native (wrapped SOL) token account.
    ///
    /// Returns `Some(rent_exempt_reserve)` if this is a native account,
    /// where the value is the rent-exempt reserve in lamports.
    /// Native accounts hold wrapped SOL where lamports == tokens.
    #[inline]
    pub fn is_native(&self) -> Option<u64> {
        if u32::from_le_bytes(self.is_native_tag) != 0 {
            Some(u64::from_le_bytes(self.is_native_val))
        } else {
            None
        }
    }

    /// Get the amount delegated to the delegate.
    ///
    /// Only meaningful if `has_delegate()` returns true.
    #[inline]
    pub fn delegated_amount(&self) -> u64 {
        u64::from_le_bytes(self.delegated_amount)
    }

    /// Check if this account has a close authority set.
    ///
    /// The close authority (if set) can close this account and recover
    /// the rent-exempt lamports.
    #[inline]
    pub fn has_close_authority(&self) -> bool {
        u32::from_le_bytes(self.close_authority_tag) != 0
    }
}

/// Writable SPL Token Account wrapper.
///
/// `TokenAccountRefMut` provides mutable access to token account data with
/// validation that the account is owned by a token program.
///
/// # Example
///
/// ```ignore
/// use solzempic::TokenAccountRefMut;
///
/// fn check_balance<'a>(accounts: &'a [AccountInfo]) -> ProgramResult {
///     let token_account = TokenAccountRefMut::load(&accounts[0])?;
///
///     let balance = token_account.amount();
///     let owner = token_account.token_owner();
///
///     Ok(())
/// }
/// ```
///
/// # When to Use
///
/// Use `TokenAccountRefMut` for:
/// - Reading token balances
/// - Verifying token account ownership
/// - Checking mint associations
/// - Working with token accounts in CPI
///
/// # ATA Creation
///
/// The [`init_ata`](Self::init_ata) method provides idempotent ATA creation
/// that skips CPI if the account is already initialized, saving ~2000 CUs.
pub struct TokenAccountRefMut<'a> {
    info: &'a AccountView,
    data: &'a mut [u8],
}

impl<'a> TokenAccountRefMut<'a> {
    /// Load a writable token account.
    ///
    /// Validates that the account is owned by a token program and is writable.
    ///
    /// # Errors
    ///
    /// * [`ProgramError::IllegalOwner`] - Not owned by Token or Token-2022
    /// * [`ProgramError::InvalidAccountData`] - Not writable or too small
    #[inline]
    pub fn load(info: &'a AccountView) -> Result<Self, ProgramError> {
        let owner = unsafe { info.owner() };
        if !address_eq(owner, &TOKEN_PROGRAM_ID) && !address_eq(owner, &TOKEN_2022_PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }
        if !info.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        let data = unsafe { info.borrow_unchecked_mut() };
        if data.len() < TokenAccountData::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self { info, data })
    }

    /// Get the underlying AccountView.
    #[inline]
    pub fn info(&self) -> &'a AccountView {
        self.info
    }

    /// Get the account's address.
    #[inline]
    pub fn address(&self) -> &Address {
        self.info.address()
    }

    /// Get a reference to the parsed token account data.
    #[inline]
    pub fn get(&self) -> &TokenAccountData {
        bytemuck::from_bytes(&self.data[..TokenAccountData::LEN])
    }

    /// Get a mutable reference to the parsed token account data.
    ///
    /// # Warning
    ///
    /// Direct mutation of token account data without going through the
    /// token program may cause inconsistencies. Use CPI for most operations.
    #[inline]
    pub fn get_mut(&mut self) -> &mut TokenAccountData {
        bytemuck::from_bytes_mut(&mut self.data[..TokenAccountData::LEN])
    }

    /// Get the mint address this account holds tokens for.
    #[inline]
    pub fn mint(&self) -> &Address {
        &self.get().mint
    }

    /// Get the owner of this token account.
    ///
    /// Note: This is the token account owner (who can transfer tokens),
    /// not the program owner (Token/Token-2022).
    #[inline]
    pub fn token_owner(&self) -> &Address {
        &self.get().owner
    }

    /// Get the token balance.
    #[inline]
    pub fn amount(&self) -> u64 {
        self.get().amount()
    }

    /// Check if this is a Token-2022 account.
    #[inline]
    pub fn is_token_2022(&self) -> bool {
        address_eq(unsafe { self.info.owner() }, &TOKEN_2022_PROGRAM_ID)
    }

    /// Reload data after CPI.
    ///
    /// Call this after any CPI that modifies the token account (transfers,
    /// mints, burns, etc.) to ensure subsequent reads see updated values.
    #[inline]
    pub fn reload(&mut self) {
        self.data = unsafe { self.info.borrow_unchecked_mut() };
    }

    /// Create or initialize an Associated Token Account idempotently.
    ///
    /// This method provides an optimized ATA creation flow:
    /// 1. If the account is already a token account, returns immediately (~10 CUs)
    /// 2. Otherwise, calls ATA program's CreateIdempotent instruction (~2000 CUs)
    ///
    /// # Arguments
    ///
    /// * `account` - The ATA to create/verify
    /// * `payer` - Account paying for rent if creation needed
    /// * `owner` - The wallet that will own the ATA
    /// * `mint` - The token mint
    /// * `system_program` - System program account
    /// * `token_program` - Token or Token-2022 program
    ///
    /// # Example
    ///
    /// ```ignore
    /// TokenAccountRefMut::init_ata(&user_ata, &payer, &user, &mint, &system_program, &token_program)?;
    /// ```
    #[inline]
    pub fn init_ata(
        account: &AccountView,
        payer: impl HasAccountView,
        owner: impl HasAccountView,
        mint: impl HasAccountView,
        system_program: impl HasAccountView,
        token_program: impl HasAccountView,
    ) -> Result<(), ProgramError> {
        let payer = payer.account_view();
        let owner = owner.account_view();
        let mint = mint.account_view();
        let system_program = system_program.account_view();
        let token_program = token_program.account_view();

        // Skip CPI if already initialized - check owner is a token program
        let account_owner = unsafe { account.owner() };
        if address_eq(account_owner, &TOKEN_PROGRAM_ID) || address_eq(account_owner, &TOKEN_2022_PROGRAM_ID) {
            return Ok(());
        }

        // Verify token program
        let token_program_id = token_program.address();
        if !address_eq(token_program_id, &TOKEN_PROGRAM_ID) && !address_eq(token_program_id, &TOKEN_2022_PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }

        let instruction_data = [1u8]; // CreateIdempotent

        let account_metas = [
            pinocchio::instruction::InstructionAccount {
                address: payer.address(),
                is_writable: true,
                is_signer: true,
            },
            pinocchio::instruction::InstructionAccount {
                address: account.address(),
                is_writable: true,
                is_signer: false,
            },
            pinocchio::instruction::InstructionAccount {
                address: owner.address(),
                is_writable: false,
                is_signer: false,
            },
            pinocchio::instruction::InstructionAccount {
                address: mint.address(),
                is_writable: false,
                is_signer: false,
            },
            pinocchio::instruction::InstructionAccount {
                address: system_program.address(),
                is_writable: false,
                is_signer: false,
            },
            pinocchio::instruction::InstructionAccount {
                address: token_program_id,
                is_writable: false,
                is_signer: false,
            },
        ];

        let instruction = pinocchio::instruction::InstructionView {
            program_id: &ASSOCIATED_TOKEN_PROGRAM_ID,
            accounts: &account_metas,
            data: &instruction_data,
        };

        // For CPI, only pass accounts that are in account_metas
        // The program is determined by instruction.program_id
        pinocchio::cpi::invoke(
            &instruction,
            &[payer, account, owner, mint, system_program, token_program],
        )?;

        Ok(())
    }

    /// Create or initialize an ATA where the owner pays for their own account.
    ///
    /// Convenience wrapper for [`init_ata`](Self::init_ata) when payer == owner.
    /// Returns the initialized token account wrapper.
    #[inline]
    pub fn init(
        account: &'a AccountView,
        owner: impl HasAccountView,
        mint: impl HasAccountView,
        system_program: impl HasAccountView,
        token_program: impl HasAccountView,
    ) -> Result<Self, ProgramError> {
        let owner = owner.account_view();
        let mint = mint.account_view();
        let system_program = system_program.account_view();
        let token_program = token_program.account_view();

        // Skip CPI if already initialized
        let account_owner = unsafe { account.owner() };
        if address_eq(account_owner, &TOKEN_PROGRAM_ID) || address_eq(account_owner, &TOKEN_2022_PROGRAM_ID) {
            return Self::load(account);
        }

        // Verify token program
        let token_program_id = token_program.address();
        if !address_eq(token_program_id, &TOKEN_PROGRAM_ID) && !address_eq(token_program_id, &TOKEN_2022_PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }

        let instruction_data = [1u8]; // CreateIdempotent

        let account_metas = [
            pinocchio::instruction::InstructionAccount { address: owner.address(), is_writable: true, is_signer: true },
            pinocchio::instruction::InstructionAccount { address: account.address(), is_writable: true, is_signer: false },
            pinocchio::instruction::InstructionAccount { address: owner.address(), is_writable: false, is_signer: false },
            pinocchio::instruction::InstructionAccount { address: mint.address(), is_writable: false, is_signer: false },
            pinocchio::instruction::InstructionAccount { address: system_program.address(), is_writable: false, is_signer: false },
            pinocchio::instruction::InstructionAccount { address: token_program_id, is_writable: false, is_signer: false },
        ];

        let instruction = pinocchio::instruction::InstructionView {
            program_id: &ASSOCIATED_TOKEN_PROGRAM_ID,
            accounts: &account_metas,
            data: &instruction_data,
        };

        pinocchio::cpi::invoke(&instruction, &[owner, account, mint, system_program, token_program])?;
        Self::load(account)
    }

    /// Initialize an ATA if it doesn't exist, otherwise load existing.
    ///
    /// Alias for [`init`](Self::init) - provided for semantic clarity when
    /// the account may or may not already exist.
    #[inline]
    pub fn init_if_needed(
        account: &'a AccountView,
        owner: impl HasAccountView,
        mint: impl HasAccountView,
        system_program: impl HasAccountView,
        token_program: impl HasAccountView,
    ) -> Result<Self, ProgramError> {
        Self::init(account, owner, mint, system_program, token_program)
    }
}

impl<'a> HasAccountView for TokenAccountRefMut<'a> {
    #[inline]
    fn account_view(&self) -> &AccountView {
        self.info
    }
}

/// Builder for TokenAccountRefMut initialization to reduce duplication when initializing
/// multiple token accounts with shared parameters.
///
/// Supports both `init` (returns wrapper) and `init_ata` (idempotent CPI).
///
/// # Example
/// ```ignore
/// // Fluent API - set payer once, reuse with different owners
/// let ata_builder = TokenAccountInitBuilder::new(&system_program).with_payer(&payer);
/// let protocol_builder = ata_builder.with_owner(&protocol);
/// let market_builder = ata_builder.with_owner(&market);
///
/// protocol_builder.init_ata(&accounts[8], &mint_a, &token_program_a)?;
/// market_builder.init_ata(&accounts[9], &mint_b, &token_program_b)?;
/// ```
#[derive(Clone, Copy)]
pub struct TokenAccountInitBuilder<'a> {
    payer: Option<&'a AccountView>,
    owner: Option<&'a AccountView>,
    system_program: &'a AccountView,
    token_program: Option<&'a AccountView>,
    token_2022_program: Option<&'a AccountView>,
}

impl<'a> TokenAccountInitBuilder<'a> {
    /// Create a new builder with just the system_program.
    /// Chain with `.with_payer()` and `.with_owner()` to set other parameters.
    #[inline]
    pub fn new<T: crate::HasAccountView>(system_program: &'a T) -> Self {
        Self {
            payer: None,
            owner: None,
            system_program: system_program.account_view(),
            token_program: None,
            token_2022_program: None,
        }
    }

    /// Create a new builder with shared payer, owner, and system_program accounts.
    /// Use this for `init_ata()` when payer differs from owner.
    #[inline]
    pub fn new_with_payer(
        payer: &'a AccountView,
        owner: &'a AccountView,
        system_program: &'a AccountView,
    ) -> Self {
        Self {
            payer: Some(payer),
            owner: Some(owner),
            system_program,
            token_program: None,
            token_2022_program: None,
        }
    }

    /// Create a new builder from a PdaInitBuilder, inheriting its payer and system_program.
    #[inline]
    pub fn from_pda_builder(pda_builder: &crate::PdaInitBuilder<'a>) -> Self {
        Self {
            payer: Some(pda_builder.payer()),
            owner: None,
            system_program: pda_builder.system_program(),
            token_program: None,
            token_2022_program: None,
        }
    }

    /// Set the payer account. Returns a new builder with payer set.
    #[inline]
    pub fn with_payer<T: crate::HasAccountView>(self, payer: &'a T) -> Self {
        Self {
            payer: Some(payer.account_view()),
            ..self
        }
    }

    /// Set the owner account. Returns a new builder with owner set.
    #[inline]
    pub fn with_owner(self, owner: &'a AccountView) -> Self {
        Self {
            owner: Some(owner),
            ..self
        }
    }

    /// Set the token programs (SPL Token and Token-2022).
    /// Returns a new builder with token programs set.
    #[inline]
    pub fn with_token_programs(
        self,
        token_program: &'a AccountView,
        token_2022_program: &'a AccountView,
    ) -> Self {
        Self {
            token_program: Some(token_program),
            token_2022_program: Some(token_2022_program),
            ..self
        }
    }

    /// Build a TokenAccountRefMut with the shared parameters plus specific mint and token_program.
    /// Uses `TokenAccountRefMut::init()` where owner pays for their own account.
    #[inline]
    pub fn build(
        &self,
        ata: &'a AccountView,
        mint: &'a AccountView,
        token_program: &'a AccountView,
    ) -> Result<TokenAccountRefMut<'a>, ProgramError> {
        let owner = self.owner.ok_or(ProgramError::NotEnoughAccountKeys)?;
        TokenAccountRefMut::init(
            ata,
            owner,
            mint,
            self.system_program,
            token_program,
        )
    }

    /// Initialize an ATA idempotently using `TokenAccountRefMut::init_ata()`.
    /// Requires both payer and owner to be set via fluent methods.
    /// Returns a loaded `TokenAccountRefMut` wrapper for the initialized account.
    ///
    /// The token program is automatically determined from the mint's owner
    /// if token programs were set via `with_token_programs()`.
    #[inline]
    pub fn init_ata(
        &self,
        ata: &'a AccountView,
        mint: impl HasAccountView,
    ) -> Result<TokenAccountRefMut<'a>, ProgramError> {
        let payer = self.payer.ok_or(ProgramError::NotEnoughAccountKeys)?;
        let owner = self.owner.ok_or(ProgramError::NotEnoughAccountKeys)?;

        // Determine token program from mint's owner
        let mint_view = mint.account_view();
        let token_program = if mint_view.owned_by(&TOKEN_2022_PROGRAM_ID) {
            self.token_2022_program.ok_or(ProgramError::NotEnoughAccountKeys)?
        } else if mint_view.owned_by(&TOKEN_PROGRAM_ID) {
            self.token_program.ok_or(ProgramError::NotEnoughAccountKeys)?
        } else {
            return Err(ProgramError::IllegalOwner);
        };

        TokenAccountRefMut::init_ata(
            ata,
            payer,
            owner,
            mint,
            self.system_program,
            token_program,
        )?;
        TokenAccountRefMut::load(ata)
    }
}
