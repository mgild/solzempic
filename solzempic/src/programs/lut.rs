//! Address Lookup Table account wrapper.
//!
//! This module provides [`Lut`], a wrapper for Address Lookup Table accounts
//! that handles both initialized and uninitialized states.

use pinocchio::{cpi::Signer, error::ProgramError, AccountView};
use solana_address::{address_eq, Address};

use super::ids::{ADDRESS_LOOKUP_TABLE_PROGRAM_ID, SYSTEM_PROGRAM_ID};
use super::traits::HasAccountView;

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
            return Ok(Self {
                info,
                initialized: false,
            });
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

    /// Create a new Address Lookup Table.
    ///
    /// # Arguments
    /// * `pda_builder` - PDA builder containing payer and system_program
    /// * `authority` - The authority account (will be the LUT owner)
    /// * `recent_slot` - Recent slot for PDA derivation
    /// * `signer_seeds` - Optional seeds if authority is a PDA
    ///
    /// # Example
    /// ```ignore
    /// let lut = Lut::wrap(&accounts[0])?;
    /// lut.create(
    ///     &pda_builder,
    ///     market.info,
    ///     lut_slot,
    ///     Some(&signer_seeds),
    /// )?;
    /// ```
    #[inline]
    pub fn create<'b>(
        &self,
        pda_builder: &crate::PdaInitBuilder<'b>,
        authority: &'b AccountView,
        recent_slot: u64,
        signer_seeds: &[pinocchio::cpi::Seed<'b>],
    ) -> Result<(), ProgramError> {
        let payer = pda_builder.payer();
        let system_program = pda_builder.system_program();

        // ALT program ID is the expected owner of the LUT account
        let alt_program_id = &ADDRESS_LOOKUP_TABLE_PROGRAM_ID;

        // Derive bump for LUT
        let (_, bump) = Address::find_program_address(
            &[authority.address().as_ref(), &recent_slot.to_le_bytes()],
            alt_program_id,
        );

        // CreateLookupTable instruction = 0
        // Data: [discriminator(4), recent_slot(8), bump(1)]
        let mut instruction_data = [0u8; 13];
        instruction_data[0..4].copy_from_slice(&0u32.to_le_bytes());
        instruction_data[4..12].copy_from_slice(&recent_slot.to_le_bytes());
        instruction_data[12] = bump;

        let account_metas = [
            pinocchio::instruction::InstructionAccount {
                address: self.info.address(),
                is_writable: true,
                is_signer: false,
            },
            pinocchio::instruction::InstructionAccount {
                address: authority.address(),
                is_writable: false,
                is_signer: true,
            },
            pinocchio::instruction::InstructionAccount {
                address: payer.address(),
                is_writable: true,
                is_signer: true,
            },
            pinocchio::instruction::InstructionAccount {
                address: system_program.address(),
                is_writable: false,
                is_signer: false,
            },
        ];

        let instruction = pinocchio::instruction::InstructionView {
            program_id: alt_program_id,
            accounts: &account_metas,
            data: &instruction_data,
        };

        let signer = Signer::from(signer_seeds);
        pinocchio::cpi::invoke_signed(
            &instruction,
            &[self.info, authority, payer, system_program],
            &[signer],
        )
    }

    /// Extend the Lookup Table with new addresses.
    ///
    /// # Arguments
    /// * `pda_builder` - PDA builder containing payer and system_program
    /// * `authority` - The authority account (must be LUT owner)
    /// * `addresses` - Slice of addresses to add to the LUT
    /// * `signer_seeds` - Optional seeds if authority is a PDA
    ///
    /// # Example
    /// ```ignore
    /// let lut = Lut::wrap(&accounts[0])?;
    /// lut.extend(
    ///     &pda_builder,
    ///     market.info,
    ///     &lut_addresses,
    ///     Some(&signer_seeds),
    /// )?;
    /// ```
    #[inline]
    pub fn extend<'b>(
        &self,
        pda_builder: &crate::PdaInitBuilder<'b>,
        authority: &'b AccountView,
        addresses: &[&Address],
        signer_seeds: &[pinocchio::cpi::Seed<'b>],
    ) -> Result<(), ProgramError> {
        // ALT program's limited_deserialize caps instruction data at 1232 bytes.
        // Per-chunk max: (1232 - 12) / 32 = 38 addresses.
        const MAX_ADDRESSES_PER_EXTEND: usize = 38;

        for chunk in addresses.chunks(MAX_ADDRESSES_PER_EXTEND) {
            self.extend_chunk(pda_builder, authority, chunk, signer_seeds)?;
        }
        Ok(())
    }

    fn extend_chunk<'b>(
        &self,
        pda_builder: &crate::PdaInitBuilder<'b>,
        authority: &'b AccountView,
        addresses: &[&Address],
        signer_seeds: &[pinocchio::cpi::Seed<'b>],
    ) -> Result<(), ProgramError> {
        let payer = pda_builder.payer();
        let system_program = pda_builder.system_program();

        let alt_program_id = &ADDRESS_LOOKUP_TABLE_PROGRAM_ID;
        // ExtendLookupTable instruction = 2
        // Data: [discriminator(4), num_addresses(8), addresses(32 * n)]
        let num_addresses = addresses.len() as u64;
        let data_len = 4 + 8 + 32 * addresses.len();

        // Stack buffer fits max chunk (38 addresses): 4 + 8 + 32*38 = 1228
        let mut data_buf = [0u8; 4 + 8 + 32 * 38];
        if data_len > data_buf.len() {
            return Err(ProgramError::InvalidArgument);
        }

        data_buf[0..4].copy_from_slice(&2u32.to_le_bytes());
        data_buf[4..12].copy_from_slice(&num_addresses.to_le_bytes());
        for (i, addr) in addresses.iter().enumerate() {
            let offset = 12 + i * 32;
            data_buf[offset..offset + 32].copy_from_slice(addr.as_ref());
        }

        let account_metas = [
            pinocchio::instruction::InstructionAccount {
                address: self.info.address(),
                is_writable: true,
                is_signer: false,
            },
            pinocchio::instruction::InstructionAccount {
                address: authority.address(),
                is_writable: false,
                is_signer: true,
            },
            pinocchio::instruction::InstructionAccount {
                address: payer.address(),
                is_writable: true,
                is_signer: true,
            },
            pinocchio::instruction::InstructionAccount {
                address: system_program.address(),
                is_writable: false,
                is_signer: false,
            },
        ];

        let instruction = pinocchio::instruction::InstructionView {
            program_id: alt_program_id,
            accounts: &account_metas,
            data: &data_buf[..data_len],
        };

        let signer = Signer::from(signer_seeds);
        pinocchio::cpi::invoke_signed(
            &instruction,
            &[self.info, authority, payer, system_program],
            &[signer],
        )
    }

    /// Create and extend a Lookup Table in a single call.
    ///
    /// # Arguments
    /// * `pda_builder` - PDA builder containing payer and system_program
    /// * `authority` - The authority account (will be the LUT owner)
    /// * `recent_slot` - Recent slot for PDA derivation
    /// * `addresses` - Slice of addresses to add to the LUT after creation
    /// * `signer_seeds` - Optional seeds if authority is a PDA
    ///
    /// # Example
    /// ```ignore
    /// let lut = Lut::wrap(&accounts[0])?;
    /// lut.init_with(
    ///     &pda_builder,
    ///     market.info,
    ///     lut_slot,
    ///     &lut_addresses,
    ///     Some(&signer_seeds),
    /// )?;
    /// ```
    #[inline]
    pub fn init_with<'b>(
        &self,
        pda_builder: &crate::PdaInitBuilder<'b>,
        authority: &'b AccountView,
        recent_slot: u64,
        addresses: &[&Address],
        signer_seeds: &[pinocchio::cpi::Seed<'b>],
    ) -> Result<(), ProgramError> {
        self.create(pda_builder, authority, recent_slot, signer_seeds)?;
        self.extend(pda_builder, authority, addresses, signer_seeds)
    }
}

impl<'a> HasAccountView for Lut<'a> {
    #[inline]
    fn account_view(&self) -> &AccountView {
        self.info
    }
}

impl<'a> TryFrom<&'a AccountView> for Lut<'a> {
    type Error = ProgramError;

    #[inline]
    fn try_from(info: &'a AccountView) -> Result<Self, Self::Error> {
        Self::wrap(info)
    }
}

impl<'a> super::traits::ValidatedAccount<'a> for Lut<'a> {
    #[inline]
    fn wrap(info: &'a AccountView) -> Result<Self, ProgramError> {
        Self::wrap(info)
    }

    #[inline]
    fn info(&self) -> &'a AccountView {
        self.info
    }
}
