//! Writable account wrapper.
//!
//! This module provides [`AccountRefMut`], a zero-overhead wrapper for mutable
//! access to program-owned accounts. It extends [`AccountRef`](super::AccountRef)
//! with write capabilities and initialization methods.

use core::marker::PhantomData;

use pinocchio::{error::ProgramError, AccountView};
use solana_address::{address_eq, Address};

use crate::{create_pda_account, Framework, Initializable, Loadable};

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
/// # Interior Mutability
///
/// `AccountRefMut` does not cache a `&mut [u8]` borrow. Instead, it re-borrows
/// from `info` on demand via `borrow_unchecked_mut()`. This means:
///
/// - Multiple `AccountRefMut` values can coexist in the same struct
/// - Data is always fresh (re-borrowed on each access)
/// - Context structs like `FillCtx` can freely hold several writable accounts
///
/// # Performance
///
/// | Operation | Cost |
/// |-----------|------|
/// | `load()` | ~50 CUs (all validations) |
/// | `get()` / `get_mut()` | ~5 CUs (pointer cast) |
/// | `init()` | ~100 CUs (validation + write discriminator) |
/// | `init_pda()` | ~2000 CUs (includes System CPI) |
///
/// # See Also
///
/// - [`AccountRef`](super::AccountRef) - Read-only version
/// - [`Initializable`] - Trait for types that can be initialized
/// - [`create_pda_account`](crate::create_pda_account) - Low-level PDA creation
pub struct AccountRefMut<'a, T: Loadable, F: Framework> {
    /// The underlying AccountView reference.
    pub info: &'a AccountView,
    /// PDA bump seed (populated when created via init_pda)
    pda_bump: Option<u8>,
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
        let data = unsafe { info.borrow_unchecked() };

        // Combined length + discriminator check (length implies non-empty,
        // so the separate is_empty check in check_discriminator is redundant)
        if data.len() < T::LEN || unsafe { *data.get_unchecked(0) } != T::DISCRIMINATOR {
            return Err(crate::errors::invalid_account_data());
        }

        Ok(Self {
            info,
            pda_bump: None,
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

    /// Get the PDA bump seed if this account was created via `init_pda`.
    ///
    /// Returns `Some(bump)` for accounts created with PDA initialization,
    /// `None` for accounts loaded with `load()` or `load_unchecked()`.
    #[inline]
    pub fn pda_bump(&self) -> Option<u8> {
        self.pda_bump
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
        let data = unsafe { self.info.borrow_unchecked() };
        // Safety: length >= T::LEN verified during load/init.
        // Account data is properly aligned on SBF.
        unsafe { &*(data.as_ptr() as *const T) }
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
        let data = unsafe { self.info.borrow_unchecked_mut() };
        // Safety: length >= T::LEN verified during load/init.
        // Account data is properly aligned on SBF.
        unsafe { &mut *(data.as_mut_ptr() as *mut T) }
    }

    /// Get the full account data slice.
    ///
    /// Returns an immutable reference to the complete account data, not just
    /// the `T::LEN` portion. Useful for accounts with variable-length data
    /// beyond the header.
    #[inline]
    pub fn data(&self) -> &[u8] {
        unsafe { self.info.borrow_unchecked() }
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
        unsafe { self.info.borrow_unchecked_mut() }
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

}

impl<'a, T: Initializable, F: Framework> AccountRefMut<'a, T, F> {
    /// Initialize an uninitialized account and wrap it.
    ///
    /// Discriminator byte 0 == uninitialized. Single borrow, no owner check
    /// (runtime enforces write-ownership).
    #[inline]
    pub fn init(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(crate::errors::account_not_writable());
        }
        let data = unsafe { info.borrow_unchecked_mut() };
        if data.len() < T::LEN {
            return Err(crate::errors::invalid_account_data());
        }
        if data[0] != 0 {
            return Err(crate::errors::account_already_initialized());
        }
        data[0] = T::DISCRIMINATOR;
        Ok(Self {
            info,
            pda_bump: None,
            _marker: PhantomData,
        })
    }

    /// Initialize if uninitialized, otherwise just load.
    ///
    /// Idempotent — safe to call multiple times on the same account.
    #[inline]
    pub fn init_if_needed(info: &'a AccountView) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(crate::errors::account_not_writable());
        }
        let data = unsafe { info.borrow_unchecked_mut() };
        if data.len() < T::LEN {
            return Err(crate::errors::invalid_account_data());
        }
        if data[0] == 0 {
            data[0] = T::DISCRIMINATOR;
        } else if data[0] != T::DISCRIMINATOR {
            return Err(crate::errors::invalid_account_data());
        }
        Ok(Self {
            info,
            pda_bump: None,
            _marker: PhantomData,
        })
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
        seeds: &[pinocchio::cpi::Seed],
        space: usize,
    ) -> Result<Self, ProgramError> {
        if !info.is_writable() {
            return Err(crate::errors::account_not_writable());
        }

        // Extract bump from seeds (last byte of last seed)
        let pda_bump = seeds
            .last()
            .and_then(|last_seed| last_seed.as_ref().last())
            .copied();

        // Create account via CPI (seeds should include bump)
        // Note: system_program param kept for API compatibility but not used
        let _ = system_program;
        create_pda_account(payer, info, &F::PROGRAM_ID, space, seeds)?;

        // Initialize: write discriminator byte
        {
            let data = unsafe { info.borrow_unchecked_mut() };
            data[0] = T::DISCRIMINATOR;
        }

        // Skip load_unchecked — CPI just created the account with the right size
        Ok(Self {
            info,
            pda_bump,
            _marker: PhantomData,
        })
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
        let data = unsafe { self.info.borrow_unchecked() };
        unsafe { &*(data.as_ptr() as *const T) }
    }

    #[inline]
    fn is_pda(&self, seeds: &[&[u8]]) -> (bool, u8) {
        let (expected, bump) = Address::find_program_address(seeds, &F::PROGRAM_ID);
        (self.info.address().as_ref() == expected.as_ref(), bump)
    }
}

impl<'a, T: Loadable, F: Framework> crate::AsAccountView for AccountRefMut<'a, T, F> {
    #[inline]
    fn as_account_view(&self) -> &AccountView {
        self.info
    }
}

impl<'a, T: Loadable, F: Framework> crate::AsAccountView for &AccountRefMut<'a, T, F> {
    #[inline]
    fn as_account_view(&self) -> &AccountView {
        self.info
    }
}

// ============================================================================
// PDA Init Builder
// ============================================================================

/// Builder for PDA initialization to reduce parameter repetition.
///
/// Captures common parameters (payer, system_program) and provides methods
/// for initializing PDAs with different seeds and sizes.
///
/// # Example
/// ```ignore
/// let pda_builder = PdaInitBuilder::new(payer, system_program);
///
/// let market_pda = Market::find_pda(mint_a, mint_b, &seed, &PROGRAM_ID);
/// let market = pda_builder.init_pda::<Market>(&accounts[5], &market_pda)?;
///
/// let vault_pda = Vault::find_pda(market, idx, &PROGRAM_ID);
/// let vault = pda_builder.init_pda::<Vault>(&accounts[6], &vault_pda)?;
/// ```

impl<'a, T: Loadable, F: Framework> crate::HasAddress for AccountRefMut<'a, T, F> {
    #[inline]
    fn address(&self) -> &Address {
        self.info.address()
    }
}

pub struct PdaInitBuilder<'a> {
    payer: &'a pinocchio::AccountView,
    system_program: &'a pinocchio::AccountView,
}

impl<'a> PdaInitBuilder<'a> {
    /// Create a new PDA init builder with shared parameters.
    #[inline]
    pub fn new<T: crate::HasAccountView, U: crate::HasAccountView>(
        payer: &'a T,
        system_program: &'a U,
    ) -> Self {
        Self {
            payer: payer.account_view(),
            system_program: system_program.account_view(),
        }
    }

    /// Get the payer account.
    #[inline]
    pub fn payer(&self) -> &'a pinocchio::AccountView {
        self.payer
    }

    /// Get the system program account.
    #[inline]
    pub fn system_program(&self) -> &'a pinocchio::AccountView {
        self.system_program
    }

    /// Initialize a PDA account with the given seeds and size.
    ///
    /// # Type Parameters
    /// * `T` - The account type to initialize (must implement Initializable)
    /// * `F` - The framework type (defaults to current framework)
    ///
    /// # Arguments
    /// * `account` - The uninitialized PDA account
    /// * `seeds` - The PDA seeds (including bump)
    ///
    /// # Example
    /// ```ignore
    /// let pda = Market::find_pda(mint_a, mint_b, &seed, &PROGRAM_ID);
    /// let market = pda_builder.init_pda::<Market>(&accounts[5], &pda.seeds())?;
    /// ```
    #[inline]
    pub fn init_pda<T, F>(
        &self,
        account: &'a pinocchio::AccountView,
        seeds: &[pinocchio::cpi::Seed],
    ) -> Result<AccountRefMut<'a, T, F>, pinocchio::error::ProgramError>
    where
        T: crate::traits::Initializable,
        F: crate::Framework,
    {
        AccountRefMut::init_pda(account, self.payer, self.system_program, seeds, T::LEN)
    }

    /// Initialize a PDA account from a typed PDA object.
    ///
    /// The account type is inferred from the PDA object's `AccountType` associated type,
    /// eliminating the need for explicit type annotations.
    ///
    /// # Type Parameters
    /// * `P` - The PDA type (must implement Pda with AccountType)
    /// * `F` - The framework type (inferred from context)
    ///
    /// # Arguments
    /// * `account` - The uninitialized PDA account
    /// * `pda` - The typed PDA object
    ///
    /// # Example
    /// ```ignore
    /// let pda = Market::find_pda(mint_a, mint_b, &seed, &PROGRAM_ID);
    /// let mut market = pda_builder.init(&accounts[5], pda)?;
    /// // Type is inferred from pda's AccountType!
    /// ```
    #[inline]
    pub fn init<P, F>(
        &self,
        account: &'a pinocchio::AccountView,
        pda: &P,
    ) -> Result<AccountRefMut<'a, P::AccountType, F>, pinocchio::error::ProgramError>
    where
        P: crate::traits::Pda,
        P::AccountType: crate::traits::Initializable,
        F: crate::Framework,
        for<'b> P::Seeds<'b>: AsRef<[pinocchio::cpi::Seed<'b>]>,
    {
        let seeds = pda.seeds();
        AccountRefMut::init_pda(
            account,
            self.payer,
            self.system_program,
            seeds.as_ref(),
            P::AccountType::LEN,
        )
    }

    /// Initialize a PDA account with a custom space allocation.
    ///
    /// Like [`init`], but allows specifying a larger `space` than `P::AccountType::LEN`.
    /// Use this for extendable account types (e.g., `PropAmmOrdersHeader`, `ActiveClmmPositionsHeader`)
    /// where the account needs room for both the header and initial data slots.
    #[inline]
    pub fn init_with_space<P, F>(
        &self,
        account: &'a pinocchio::AccountView,
        pda: &P,
        space: usize,
    ) -> Result<AccountRefMut<'a, P::AccountType, F>, pinocchio::error::ProgramError>
    where
        P: crate::traits::Pda,
        P::AccountType: crate::traits::Initializable,
        F: crate::Framework,
        for<'b> P::Seeds<'b>: AsRef<[pinocchio::cpi::Seed<'b>]>,
    {
        let seeds = pda.seeds();
        AccountRefMut::init_pda(
            account,
            self.payer,
            self.system_program,
            seeds.as_ref(),
            space,
        )
    }

    /// Initialize a PDA account from a value that converts to a PDA.
    ///
    /// This accepts anything that implements `Into<P>`, allowing you to pass
    /// tuples or other types that convert to PDAs without explicit construction.
    ///
    /// # Example
    /// ```ignore
    /// // Pass a tuple that converts to ActiveClmmPda then to ActiveClmmPositionsPda
    /// let active_clmm: AccountRefMut<ActiveClmmPositionsHeader> =
    ///     pda_builder.init_into(&accounts[10], (market_key, 0).into())?;
    /// ```
    #[inline]
    pub fn init_into<P, F, I>(
        &self,
        account: &'a pinocchio::AccountView,
        pda: I,
    ) -> Result<AccountRefMut<'a, P::AccountType, F>, pinocchio::error::ProgramError>
    where
        I: Into<P>,
        P: crate::traits::Pda,
        P::AccountType: crate::traits::Initializable,
        F: crate::Framework,
        for<'b> P::Seeds<'b>: AsRef<[pinocchio::cpi::Seed<'b>]>,
    {
        let pda = pda.into();
        self.init(account, &pda)
    }

    /// Initialize a PDA account from a tuple, inferring the PDA type from the return type.
    ///
    /// This is a convenience method for accounts that implement a tuple-to-PDA conversion.
    /// The account type is inferred from the return type annotation.
    ///
    /// # Example
    /// ```ignore
    /// let active_clmm: AccountRefMut<ActiveClmmPositionsHeader> =
    ///     pda_builder.init_tuple(&accounts[10], (market_key, 0))?;
    /// ```
    #[inline]
    pub fn init_tuple<T, F, P>(
        &self,
        account: &'a pinocchio::AccountView,
        tuple: (pinocchio::Address, u64),
    ) -> Result<AccountRefMut<'a, T, F>, pinocchio::error::ProgramError>
    where
        T: crate::traits::Initializable + crate::traits::FromTuplePda<Pda = P>,
        P: crate::traits::Pda<AccountType = T>,
        F: crate::Framework,
        for<'b> P::Seeds<'b>: AsRef<[pinocchio::cpi::Seed<'b>]>,
    {
        let pda = T::from_tuple(tuple);
        self.init(account, &pda)
    }

    /// Create a PDA account without initializing (for custom initialization logic).
    ///
    /// This method creates the account and allocates space, but does not initialize
    /// the discriminator or data. Use this when the account needs custom initialization
    /// logic that can't be expressed with the `Initializable` trait.
    ///
    /// # Arguments
    /// * `account` - The uninitialized PDA account
    /// * `pda` - The typed PDA object providing seeds
    /// * `program_id` - The program ID that will own the account
    /// * `space` - The space to allocate for the account
    ///
    /// # Example
    /// ```ignore
    /// let pda = ActiveClmmPositions::find_pda(market, 0, &PROGRAM_ID);
    /// pda_builder.create(&accounts[10], &pda, &PROGRAM_ID, ActiveClmmPositions::account_size(100))?;
    /// // Now manually initialize the account data...
    /// ```
    #[inline]
    pub fn create<P>(
        &self,
        account: &'a pinocchio::AccountView,
        pda: &P,
        program_id: &solana_address::Address,
        space: usize,
    ) -> Result<(), pinocchio::error::ProgramError>
    where
        P: crate::traits::Pda,
        for<'b> P::Seeds<'b>: AsRef<[pinocchio::cpi::Seed<'b>]>,
    {
        let seeds = pda.seeds();
        let seeds_slice = seeds.as_ref();

        crate::create_pda_account(self.payer, account, program_id, space, seeds_slice)
    }
}
