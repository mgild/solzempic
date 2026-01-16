//! Writable account wrapper.
//!
//! This module provides [`AccountRefMut`], a zero-overhead wrapper for mutable
//! access to program-owned accounts. It extends [`AccountRef`](super::AccountRef)
//! with write capabilities and initialization methods.

use core::marker::PhantomData;

use pinocchio::{error::ProgramError, AccountView};
use solana_address::{Address, address_eq};

use crate::{check_discriminator, create_pda_account, Framework, Initializable, Loadable, SYSTEM_PROGRAM_ID};

use super::traits::AsAccountRef;

/// Writable account wrapper for typed account data.
///
/// `AccountRefMut` provides safe, zero-copy read and write access to account data.
/// It performs all the validations of [`AccountRef`](super::AccountRef) plus an
/// additional `is_writable` check.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the borrowed AccountInfo
/// * `T` - The account data type (must implement [`Loadable`])
/// * `F` - The framework type (must implement [`Framework`](crate::Framework))
///
/// # Validation on Load
///
/// When calling [`load`](Self::load), the following checks are performed:
///
/// 1. **Writable**: Account must have `is_writable == true`
/// 2. **Ownership**: Account must be owned by `F::PROGRAM_ID`
/// 3. **Size**: Account data must be at least `T::LEN` bytes
/// 4. **Discriminator**: First bytes must match `T::DISCRIMINATOR`
///
/// # Initialization Methods
///
/// For types implementing [`Initializable`], additional methods are available:
///
/// | Method | Use Case |
/// |--------|----------|
/// | [`init`](Self::init) | Initialize a new account |
/// | [`init_if_needed`](Self::init_if_needed) | Initialize only if not already initialized |
/// | [`init_pda`](Self::init_pda) | Create PDA and initialize in one call |
///
/// # Example
///
/// ```ignore
/// use solzempic::AccountRefMut;
///
/// // Load an existing writable account
/// let mut counter: AccountRefMut<Counter> = AccountRefMut::load(&accounts[0])?;
///
/// // Modify the data
/// counter.get_mut().count += 1;
///
/// // Or initialize a new account
/// let mut new_account: AccountRefMut<Counter> = AccountRefMut::init(
///     &accounts[1],
///     CounterParams { initial_count: 0 },
/// )?;
/// ```
///
/// # Post-CPI Reload
///
/// After any CPI that might modify the account, call [`reload`](Self::reload)
/// to refresh the data reference:
///
/// ```ignore
/// // Perform CPI that modifies account...
/// invoke(&instruction, &account_infos)?;
///
/// // Refresh our view of the data
/// account.reload();
/// ```
///
/// # Performance
///
/// | Operation | Cost |
/// |-----------|------|
/// | `load()` | ~50 CUs (all validations) |
/// | `get()` / `get_mut()` | ~5 CUs (pointer cast) |
/// | `init()` | ~100 CUs (validation + write discriminator) |
/// | `init_pda()` | ~2000 CUs (includes System CPI) |
/// | `reload()` | ~10 CUs (re-borrow) |
///
/// # See Also
///
/// - [`AccountRef`](super::AccountRef) - Read-only version
/// - [`Initializable`] - Trait for types that can be initialized
/// - [`create_pda_account`](crate::create_pda_account) - Low-level PDA creation
pub struct AccountRefMut<'a, T: Loadable, F: Framework> {
    /// The underlying AccountView reference.
    pub info: &'a AccountView,
    data: &'a mut [u8],
    _marker: PhantomData<(T, F)>,
}

impl<'a, T: Loadable, F: Framework> AccountRefMut<'a, T, F> {
    /// Load and validate an already-initialized writable account.
    ///
    /// This is the primary way to create an `AccountRefMut` for existing accounts.
    /// It performs full validation including the `is_writable` check.
    ///
    /// # Validation
    ///
    /// 1. Account must be writable (`is_writable == true`)
    /// 2. Account owner must equal `F::PROGRAM_ID`
    /// 3. Account data must be at least `T::LEN` bytes
    /// 4. Account discriminator must match `T::DISCRIMINATOR`
    ///
    /// # Arguments
    ///
    /// * `info` - The AccountInfo to wrap (must be writable)
    ///
    /// # Errors
    ///
    /// * [`ProgramError::InvalidAccountData`] - Account not writable
    /// * [`ProgramError::IllegalOwner`] - Account not owned by this program
    /// * [`ProgramError::InvalidAccountData`] - Data too small or wrong discriminator
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut user: AccountRefMut<User> = AccountRefMut::load(&accounts[0])?;
    /// user.get_mut().balance += deposit_amount;
    /// ```
    #[inline]
    pub fn load(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(crate::errors::account_not_writable());
        }
        if !address_eq(unsafe { info.owner() }, &F::PROGRAM_ID) {
            return Err(ProgramError::IllegalOwner);
        }
        Self::load_unchecked(info)
    }

    /// Try to load an account, returning `None` if validation fails.
    ///
    /// This is useful for optional accounts that may or may not exist (e.g.,
    /// opposite side orderbook shards that haven't been initialized yet).
    /// Returns `None` for:
    /// - System-owned accounts (uninitialized PDAs)
    /// - Accounts not owned by this program
    /// - Non-writable accounts
    /// - Accounts with wrong discriminator
    ///
    /// # Returns
    ///
    /// `Some(Self)` if the account is valid and initialized, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(mut shard) = AccountRefMut::<OrderShard>::try_load(&accounts[0]) {
    ///     // Shard exists and is valid, use it
    ///     let best_price = shard.get().best_price();
    /// } else {
    ///     // Shard doesn't exist yet, skip crossing check
    /// }
    /// ```
    #[inline]
    pub fn try_load(info: &'a AccountView) -> Option<Self> {
        if !info.is_writable() {
            return None;
        }
        if !address_eq(unsafe { info.owner() }, &F::PROGRAM_ID) {
            return None;
        }
        Self::load_unchecked(info).ok()
    }

    /// Load an account without ownership or writable validation.
    ///
    /// This skips both the `is_writable` and ownership checks, but still validates
    /// data size and discriminator. Use with caution - this is primarily for
    /// advanced use cases like cross-program account manipulation.
    ///
    /// # Warning
    ///
    /// Only use this if you have a specific reason to skip validation.
    /// Writing to a read-only account will cause a runtime error.
    ///
    /// # Errors
    ///
    /// * [`ProgramError::InvalidAccountData`] - Data too small or wrong discriminator
    #[inline]
    pub fn load_unchecked(info: &'a AccountView) -> Result<Self, ProgramError> {
        let data = unsafe { info.borrow_unchecked_mut() };

        if data.len() < T::LEN {
            return Err(crate::errors::invalid_account_data());
        }

        if !check_discriminator(data, T::DISCRIMINATOR) {
            return Err(crate::errors::invalid_account_data());
        }

        Ok(Self {
            info,
            data,
            _marker: PhantomData,
        })
    }

    /// Get the account's address.
    ///
    /// Convenience method equivalent to `self.info.address()`.
    #[inline]
    pub fn address(&self) -> &Address {
        self.info.address()
    }

    /// Get a reference to the parsed account data.
    ///
    /// Returns a typed reference to the account's data via zero-copy pointer cast.
    /// For mutable access, use [`get_mut`](Self::get_mut) instead.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let balance = account.get().balance;
    /// ```
    #[inline]
    pub fn get(&self) -> &T {
        bytemuck::from_bytes(&self.data[..T::LEN])
    }

    /// Get a mutable reference to the parsed account data.
    ///
    /// Returns a typed mutable reference, allowing direct modification of
    /// the account's on-chain data. Changes are written immediately to the
    /// account's data buffer.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let data = account.get_mut();
    /// data.balance += amount;
    /// data.last_update = current_slot;
    /// ```
    ///
    /// # Note
    ///
    /// Modifications are reflected in the account's underlying data immediately.
    /// There's no need for an explicit "save" or "commit" operation.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        bytemuck::from_bytes_mut(&mut self.data[..T::LEN])
    }

    /// Get the full account data slice.
    ///
    /// Returns an immutable reference to the complete account data, not just
    /// the `T::LEN` portion. Useful for accounts with variable-length data
    /// beyond the header.
    #[inline]
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get the full account data slice mutably.
    ///
    /// Returns a mutable reference to the complete account data. Useful for
    /// accounts with variable-length data (like order arrays) that need to
    /// create views spanning header + data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For accounts with variable-length orders after the header
    /// let mut orders = OrdersView::from_account(account.data_mut()).unwrap();
    /// ```
    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.data
    }

    /// Reload data reference after CPI.
    ///
    /// After any CPI that might modify this account's data, call this method
    /// to refresh the internal data reference. This ensures subsequent reads
    /// see the updated state.
    ///
    /// # When to Call
    ///
    /// Call `reload()` after:
    /// - Any `invoke()` or `invoke_signed()` that includes this account
    /// - Token transfers to/from this account
    /// - Any external program modification
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Transfer tokens to vault
    /// token_program::transfer(&source, &vault, &authority, amount)?;
    ///
    /// // Refresh our view before reading updated balance
    /// vault.reload();
    /// let new_balance = vault.get().amount();
    /// ```
    #[inline]
    pub fn reload(&mut self) {
        self.data = unsafe { self.info.borrow_unchecked_mut() };
    }

    /// Check if this account is a PDA derived from the given seeds.
    ///
    /// Derives the expected PDA address from the seeds and framework's program ID,
    /// then compares it against this account's address.
    ///
    /// # Arguments
    ///
    /// * `seeds` - The PDA seeds (without the bump)
    ///
    /// # Returns
    ///
    /// A tuple of `(is_valid, bump)`:
    /// - `is_valid` - `true` if the account address matches the derived PDA
    /// - `bump` - The canonical bump seed for this derivation
    ///
    /// # Performance
    ///
    /// PDA derivation is expensive (~2000 CUs). For frequent validation, consider
    /// storing the bump in the account data itself.
    #[inline]
    pub fn is_pda(&self, seeds: &[&[u8]]) -> (bool, u8) {
        let (expected, bump) = Address::find_program_address(seeds, &F::PROGRAM_ID);
        (self.info.address().as_ref() == expected.as_ref(), bump)
    }

    /// Check if an account is uninitialized and can be initialized.
    ///
    /// An account is considered uninitialized if:
    /// - It's owned by the System program (fresh account), OR
    /// - It's owned by this program AND has a zero discriminator
    ///
    /// This is used internally by [`init`](Self::init) and [`init_if_needed`](Self::init_if_needed).
    #[inline]
    fn is_uninit(info: &AccountView) -> bool {
        let owner = unsafe { info.owner() };
        if address_eq(owner, &SYSTEM_PROGRAM_ID) {
            return true;
        }

        if address_eq(owner, &F::PROGRAM_ID) {
            let data = unsafe { info.borrow_unchecked() };
            return data.is_empty() || data[0] == 0;
        }

        false
    }
}

impl<'a, T: Initializable, F: Framework> AccountRefMut<'a, T, F> {
    /// Initialize an uninitialized account and wrap it.
    ///
    /// This method writes the type's discriminator to an uninitialized account,
    /// then returns a wrapper for the initialized account. All other bytes
    /// remain zeroed.
    ///
    /// # Preconditions
    ///
    /// - Account must be writable
    /// - Account must be uninitialized (system-owned or zero discriminator)
    /// - Account must already have sufficient space allocated
    ///
    /// # Arguments
    ///
    /// * `info` - The uninitialized account to initialize
    ///
    /// # Errors
    ///
    /// * [`ProgramError::InvalidAccountData`] - Account not writable or too small
    /// * [`ProgramError::AccountAlreadyInitialized`] - Account already has data
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Account was created with sufficient space beforehand
    /// let mut counter: AccountRefMut<Counter> = AccountRefMut::init(&accounts[0])?;
    /// counter.get_mut().owner = *owner.key();
    /// counter.get_mut().count = 0;
    /// ```
    ///
    /// # See Also
    ///
    /// - [`init_pda`](Self::init_pda) - Create and initialize PDA in one call
    /// - [`init_if_needed`](Self::init_if_needed) - Idempotent initialization
    #[inline]
    pub fn init(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(crate::errors::account_not_writable());
        }
        if !Self::is_uninit(info) {
            return Err(crate::errors::account_already_initialized());
        }
        let data = unsafe { info.borrow_unchecked_mut() };
        if data.len() < T::LEN {
            return Err(crate::errors::invalid_account_data());
        }
        // Write discriminator byte
        data[0] = T::DISCRIMINATOR;
        Self::load_unchecked(info)
    }

    /// Initialize if uninitialized, otherwise just load.
    ///
    /// This is an idempotent initialization method - it's safe to call multiple
    /// times on the same account. If the account is already initialized, it
    /// simply loads and returns it without modification.
    ///
    /// # Use Cases
    ///
    /// - Creating user accounts on first interaction
    /// - Ensuring an account exists before use
    /// - Defensive programming where initialization state is uncertain
    ///
    /// # Arguments
    ///
    /// * `info` - The account to initialize or load
    ///
    /// # Errors
    ///
    /// * [`ProgramError::InvalidAccountData`] - Account not writable or too small
    /// * [`ProgramError::InvalidAccountData`] - Data too small or wrong discriminator (if already init)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Safe to call even if user already has an account
    /// let mut user: AccountRefMut<User> = AccountRefMut::init_if_needed(user_account)?;
    /// if user.get().owner == [0u8; 32] {
    ///     user.get_mut().owner = *owner.key();
    /// }
    /// ```
    #[inline]
    pub fn init_if_needed(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(crate::errors::account_not_writable());
        }
        if Self::is_uninit(info) {
            let data = unsafe { info.borrow_unchecked_mut() };
            if data.len() < T::LEN {
                return Err(crate::errors::invalid_account_data());
            }
            // Write discriminator byte
            data[0] = T::DISCRIMINATOR;
        }
        Self::load_unchecked(info)
    }

    /// Create a PDA account and initialize it in one operation.
    ///
    /// This combines [`create_pda_account`](crate::create_pda_account) and
    /// [`init`](Self::init) into a single convenient method. Use this when
    /// creating new program-owned accounts that are derived from seeds.
    ///
    /// # Arguments
    ///
    /// * `info` - The PDA account to create and initialize
    /// * `payer` - The account paying for rent (must be a signer)
    /// * `system_program` - The System program (kept for API compatibility)
    /// * `seeds` - The PDA seeds **including the bump seed**
    /// * `space` - The space to allocate (should be `T::LEN` or larger)
    ///
    /// # Errors
    ///
    /// * [`ProgramError::InvalidAccountData`] - Account not writable
    /// * System program errors - Insufficient funds, wrong address, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Derive PDA seeds
    /// let (_, bump) = Pubkey::find_program_address(
    ///     &[b"market", base_mint.as_ref(), quote_mint.as_ref()],
    ///     &program_id,
    /// );
    /// let seeds: &[&[u8]] = &[
    ///     b"market",
    ///     base_mint.as_ref(),
    ///     quote_mint.as_ref(),
    ///     &[bump],
    /// ];
    ///
    /// // Create and initialize in one call
    /// let mut market: AccountRefMut<Market> = AccountRefMut::init_pda(
    ///     market_account,
    ///     payer.info(),
    ///     system_program.info(),
    ///     seeds,
    ///     Market::LEN,
    /// )?;
    /// market.get_mut().base_mint = *base_mint;
    /// market.get_mut().quote_mint = *quote_mint;
    /// ```
    ///
    /// # Performance
    ///
    /// This method invokes the System program (~2000 CUs for account creation).
    ///
    /// # See Also
    ///
    /// - [`create_pda_account`](crate::create_pda_account) - Low-level PDA creation
    /// - [`init`](Self::init) - Initialize pre-existing account
    #[inline]
    pub fn init_pda(
        info: &'a AccountView,
        payer: &AccountView,
        system_program: &AccountView,
        seeds: &[&[u8]],
        space: usize,
    ) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(crate::errors::account_not_writable());
        }

        // Create account via CPI (seeds should include bump)
        // Note: system_program param kept for API compatibility but not used
        let _ = system_program;
        create_pda_account(payer, info, &F::PROGRAM_ID, space, seeds)?;

        // Initialize: write discriminator byte
        let data = unsafe { info.borrow_unchecked_mut() };
        data[0] = T::DISCRIMINATOR;
        Self::load_unchecked(info)
    }
}

impl<'a, T: Loadable, F: Framework> AsAccountRef<'a, T, F> for AccountRefMut<'a, T, F> {
    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }

    #[inline]
    fn address(&self) -> &Address {
        self.info.address()
    }

    #[inline]
    fn get(&self) -> &T {
        bytemuck::from_bytes(&self.data[..T::LEN])
    }

    #[inline]
    fn is_pda(&self, seeds: &[&[u8]]) -> (bool, u8) {
        let (expected, bump) = Address::find_program_address(seeds, &F::PROGRAM_ID);
        (self.info.address().as_ref() == expected.as_ref(), bump)
    }
}
