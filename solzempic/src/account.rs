//! Account utilities for creating and managing Solana accounts.
//!
//! This module provides low-level utilities for working with Solana accounts:
//!
//! - [`rent_exempt_minimum`]: Calculate rent-exempt balance for account sizes
//! - [`transfer_lamports`]: Transfer SOL between accounts via System program
//! - [`create_pda_account`]: Create PDA accounts with proper signing
//!
//! # Performance
//!
//! All functions in this module are `#[inline]` and compile to minimal
//! instructions. The rent calculation is `const fn` and evaluated at compile
//! time when possible.
//!
//! # Example
//!
//! ```ignore
//! use solzempic::{create_pda_account, rent_exempt_minimum, transfer_lamports};
//!
//! // Calculate rent for a 100-byte account
//! let rent = rent_exempt_minimum(100);
//!
//! // Transfer SOL
//! transfer_lamports(&from, &to, &system_program, 1_000_000)?;
//!
//! // Create a PDA account
//! let seeds = &[b"my_account", user.key().as_ref(), &[bump]];
//! create_pda_account(&payer, &new_account, &program_id, 256, seeds)?;
//! ```

use pinocchio::{
    AccountView,
    cpi::{invoke_signed, Seed, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
};
use solana_address::Address;

use crate::SYSTEM_PROGRAM_ID;

/// Maximum account size on Solana (10 MB).
///
/// This is a hard limit enforced by the Solana runtime. Attempting to create
/// or resize an account beyond this size will fail.
///
/// For programs needing more storage, consider sharding data across multiple
/// accounts (see [`ShardRefContext`](crate::ShardRefContext)).
pub const MAX_ACCOUNT_SIZE: usize = 10 * 1024 * 1024;

/// Approximate lamports required per byte for rent exemption.
///
/// This value is based on the current Solana rent rate (~0.00000696 SOL per byte).
/// For exact calculations in production code, prefer reading the Rent sysvar
/// directly using [`RentSysvar`](crate::RentSysvar).
///
/// # Note
///
/// This constant is used internally by [`rent_exempt_minimum`] and may need
/// adjustment if Solana's rent parameters change significantly.
pub const LAMPORTS_PER_BYTE: u64 = 6960;

/// Calculate the minimum rent-exempt balance for an account of the given size.
///
/// This is a compile-time approximation suitable for most use cases. The
/// calculation includes the 128-byte account metadata overhead that Solana
/// adds to all accounts.
///
/// # Arguments
///
/// * `data_len` - The size of the account's data in bytes (not including metadata)
///
/// # Returns
///
/// The approximate minimum lamports required to make the account rent-exempt.
///
/// # Example
///
/// ```ignore
/// // Calculate rent for a 256-byte account
/// let rent = rent_exempt_minimum(256);
/// assert!(rent > 0);
///
/// // The result can be used at compile time
/// const MY_ACCOUNT_RENT: u64 = rent_exempt_minimum(100);
/// ```
///
/// # Accuracy
///
/// This approximation is within 1% of the exact value for typical account
/// sizes. For critical applications where exact lamport counts matter,
/// read the Rent sysvar at runtime.
#[inline]
pub const fn rent_exempt_minimum(data_len: usize) -> u64 {
    // Base rent (128 bytes for account metadata) + data
    let total_bytes = 128 + data_len;
    (total_bytes as u64) * LAMPORTS_PER_BYTE
}

/// Transfer lamports (SOL) between accounts using the System program.
///
/// This function performs a CPI to the System program's Transfer instruction.
/// The source account must be a signer on the transaction.
///
/// # Arguments
///
/// * `from` - The account to transfer from (must be a signer)
/// * `to` - The account to transfer to (must be writable)
/// * `system_program` - The System program account
/// * `amount` - Number of lamports to transfer
///
/// # Returns
///
/// * `Ok(())` - Transfer succeeded (or amount was 0)
/// * `Err(ProgramError)` - Transfer failed (insufficient funds, invalid accounts, etc.)
///
/// # Example
///
/// ```ignore
/// use solzempic::transfer_lamports;
///
/// // Transfer 0.001 SOL (1,000,000 lamports)
/// transfer_lamports(
///     user.info(),
///     treasury.info(),
///     system_program.info(),
///     1_000_000,
/// )?;
/// ```
///
/// # Performance
///
/// - Zero-amount transfers return immediately without CPI (~10 CUs)
/// - Non-zero transfers invoke System program (~150 CUs)
///
/// # Note
///
/// For transferring tokens (SPL), use the Token program's Transfer instruction
/// instead. This function only handles native SOL transfers.
#[inline]
pub fn transfer_lamports<'a>(
    from: &'a AccountView,
    to: &'a AccountView,
    system_program: &'a AccountView,
    amount: u64,
) -> Result<(), ProgramError> {
    if amount == 0 {
        return Ok(());
    }

    // System program transfer instruction
    // Discriminator: 2 (Transfer)
    let mut instruction_data = [0u8; 12];
    instruction_data[0..4].copy_from_slice(&2u32.to_le_bytes()); // Transfer discriminator
    instruction_data[4..12].copy_from_slice(&amount.to_le_bytes());

    let account_metas = [
        InstructionAccount {
            address: from.address(),
            is_writable: true,
            is_signer: true,
        },
        InstructionAccount {
            address: to.address(),
            is_writable: true,
            is_signer: false,
        },
    ];

    let instruction = InstructionView {
        program_id: &SYSTEM_PROGRAM_ID,
        accounts: &account_metas,
        data: &instruction_data,
    };

    let account_infos = &[from, to, system_program];
    pinocchio::cpi::invoke(&instruction, account_infos)
}

/// Create a Program Derived Address (PDA) account with the specified size.
///
/// This function performs a CPI to the System program's CreateAccount instruction,
/// with the PDA signing for itself using the provided seeds. The account is created
/// with sufficient lamports to be rent-exempt.
///
/// # Arguments
///
/// * `payer` - The account paying for rent (must be a signer with sufficient lamports)
/// * `new_account` - The PDA account to create (address must match seeds + program_id)
/// * `program_id` - The program that will own the created account
/// * `space` - The size of the account data in bytes
/// * `seeds` - The PDA seeds **including the bump seed** (max 6 seeds)
///
/// # Returns
///
/// * `Ok(())` - Account created successfully
/// * `Err(ProgramError)` - Creation failed (insufficient funds, wrong address, etc.)
///
/// # Example
///
/// ```ignore
/// use solzempic::create_pda_account;
///
/// // Find PDA and get bump
/// let (pda, bump) = Pubkey::find_program_address(
///     &[b"user", owner.key().as_ref()],
///     &program_id,
/// );
///
/// // Include bump in seeds when creating
/// let seeds: &[&[u8]] = &[b"user", owner.key().as_ref(), &[bump]];
/// create_pda_account(&payer, &pda_account, &program_id, 256, seeds)?;
/// ```
///
/// # Seed Limits
///
/// This function supports up to **6 seeds**. This accommodates most PDA
/// derivation patterns:
///
/// ```ignore
/// // Common patterns (3-4 seeds):
/// &[b"market", token_a.as_ref(), token_b.as_ref(), &[bump]]
///
/// // Complex patterns (5-6 seeds):
/// &[b"order", market.as_ref(), user.as_ref(), &order_id.to_le_bytes(), &[bump]]
/// ```
///
/// # Important
///
/// - The `new_account` address **must** be derivable from `seeds` + `program_id`
/// - The bump seed should typically be the last seed
/// - Use [`pinocchio::pubkey::find_program_address`] to derive the PDA and bump
///
/// # Performance
///
/// This function invokes the System program (~2000 CUs total).
///
/// # See Also
///
/// - [`AccountRefMut::init_pda`](crate::AccountRefMut::init_pda) - Higher-level
///   wrapper that creates and initializes in one call
#[inline]
pub fn create_pda_account<'a>(
    payer: &'a AccountView,
    new_account: &'a AccountView,
    program_id: &Address,
    space: usize,
    seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let lamports = rent_exempt_minimum(space);

    // CreateAccount instruction
    // Discriminator: 0
    let mut instruction_data = [0u8; 52];
    instruction_data[0..4].copy_from_slice(&0u32.to_le_bytes()); // CreateAccount discriminator
    instruction_data[4..12].copy_from_slice(&lamports.to_le_bytes());
    instruction_data[12..20].copy_from_slice(&(space as u64).to_le_bytes());
    instruction_data[20..52].copy_from_slice(program_id.as_ref());

    let account_metas = [
        InstructionAccount {
            address: payer.address(),
            is_writable: true,
            is_signer: true,
        },
        InstructionAccount {
            address: new_account.address(),
            is_writable: true,
            is_signer: true,
        },
    ];

    let instruction = InstructionView {
        program_id: &SYSTEM_PROGRAM_ID,
        accounts: &account_metas,
        data: &instruction_data,
    };

    // Only pass the 2 accounts referenced by the instruction (matches pinocchio-system)
    let account_infos = &[payer, new_account];

    // Convert &[&[u8]] to [Seed] for invoke_signed
    // Support up to 6 seeds (e.g., ["market", token_a, token_b, seed, bump, extra])
    let seed_refs: [Seed; 6] = [
        Seed::from(seeds.first().copied().unwrap_or(&[])),
        Seed::from(seeds.get(1).copied().unwrap_or(&[])),
        Seed::from(seeds.get(2).copied().unwrap_or(&[])),
        Seed::from(seeds.get(3).copied().unwrap_or(&[])),
        Seed::from(seeds.get(4).copied().unwrap_or(&[])),
        Seed::from(seeds.get(5).copied().unwrap_or(&[])),
    ];
    let seed_count = seeds.len().min(6);
    let signer = Signer::from(&seed_refs[..seed_count]);
    invoke_signed(&instruction, account_infos, &[signer])
}
