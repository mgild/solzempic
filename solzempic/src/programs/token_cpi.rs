//! Token CPI Operations
//!
//! CPI helpers for SPL Token and Token-2022 operations including transfers,
//! account creation, and mint/burn operations.

use pinocchio::{cpi::Signer, error::ProgramError, AccountView};
use solana_address::Address;

pub use pinocchio::cpi::Seed;

use super::ids::{
    ASSOCIATED_TOKEN_PROGRAM_ID, PTOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID,
};
use crate::AsAccountView;

// ============================================================================
// CPI Helpers
// ============================================================================

/// Helper to invoke or invoke_signed based on whether signer seeds are provided
#[inline]
fn invoke_maybe_signed<'a, const N: usize>(
    instruction: &pinocchio::instruction::InstructionView<'_, '_, '_, '_>,
    account_infos: &[&'a AccountView; N],
    signer_seeds: &[Seed<'a>],
) -> Result<(), ProgramError> {
    if signer_seeds.is_empty() {
        pinocchio::cpi::invoke(instruction, account_infos)
    } else {
        let signer = Signer::from(signer_seeds);
        pinocchio::cpi::invoke_signed(instruction, account_infos, &[signer])
    }
}

// ============================================================================
// Transfer Operations
// ============================================================================

/// Transfer tokens using the appropriate token program
///
/// The token program is automatically derived from the `from` account's owner,
/// which must be either SPL Token or Token-2022 program.
///
/// Pass `&[]` for `signer_seeds` if the authority is a regular signer (not a PDA).
///
/// Accepts any type that can be converted to `&AccountView` via `AsAccountView`.
#[inline]
pub fn transfer<'a>(
    from: impl AsAccountView,
    to: impl AsAccountView,
    authority: impl AsAccountView,
    amount: u64,
    signer_seeds: &[Seed<'a>],
) -> Result<(), ProgramError> {
    // Skip transfer if amount is zero
    if amount == 0 {
        return Ok(());
    }

    let from = from.as_account_view();
    let to = to.as_account_view();
    let authority = authority.as_account_view();

    // Derive token program from the from account's owner
    // SAFETY: We only read the owner field to identify the token program.
    // The owner is not modified during this transfer operation.
    let token_program = unsafe { from.owner() };

    // Build instruction data: [3] for Transfer
    let mut instruction_data = [0u8; 9];
    instruction_data[0] = 3; // Transfer instruction
    instruction_data[1..9].copy_from_slice(&amount.to_le_bytes());

    let account_metas = [
        pinocchio::instruction::InstructionAccount {
            address: from.address(),
            is_writable: true,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: to.address(),
            is_writable: true,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: authority.address(),
            is_writable: false,
            is_signer: true,
        },
    ];

    let instruction = pinocchio::instruction::InstructionView {
        program_id: token_program,
        accounts: &account_metas,
        data: &instruction_data,
    };

    // Only pass accounts that match account_metas - token_program is identified via program_id
    invoke_maybe_signed(&instruction, &[from, to, authority], signer_seeds)
}

// ============================================================================
// Account Creation
// ============================================================================

/// Initialize a token account (assumes account is already allocated)
#[inline]
pub fn initialize_account<'a>(
    account: &'a AccountView,
    mint: &'a AccountView,
    owner: &'a AccountView,
    rent_sysvar: &'a AccountView,
) -> Result<(), ProgramError> {
    // InitializeAccount instruction = 1
    let instruction_data = [1u8];

    let account_metas = [
        pinocchio::instruction::InstructionAccount {
            address: account.address(),
            is_writable: true,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: mint.address(),
            is_writable: false,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: owner.address(),
            is_writable: false,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: rent_sysvar.address(),
            is_writable: false,
            is_signer: false,
        },
    ];

    // SAFETY: We only read the owner field to identify the token program.
    let token_program = unsafe { account.owner() };

    let instruction = pinocchio::instruction::InstructionView {
        program_id: token_program,
        accounts: &account_metas,
        data: &instruction_data,
    };

    pinocchio::cpi::invoke(&instruction, &[account, mint, owner, rent_sysvar])
}

/// Create associated token account via CPI
/// This will fail if account already exists
#[inline]
pub fn create_associated_token_account<'a>(
    payer: &'a AccountView,
    owner: &'a AccountView,
    mint: &'a AccountView,
    token_account: &'a AccountView,
    system_program: &'a AccountView,
    ata_program: &'a AccountView,
) -> Result<(), ProgramError> {
    // SAFETY: We only read the owner field to identify the token program.
    let token_program = unsafe { mint.owner() };

    // ATA Create instruction has no data
    let account_metas = [
        pinocchio::instruction::InstructionAccount {
            address: payer.address(),
            is_writable: true,
            is_signer: true,
        },
        pinocchio::instruction::InstructionAccount {
            address: token_account.address(),
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
            address: token_program,
            is_writable: false,
            is_signer: false,
        },
    ];

    let instruction = pinocchio::instruction::InstructionView {
        program_id: ata_program.address(),
        accounts: &account_metas,
        data: &[],
    };

    // For CPI, only pass accounts that are in account_metas
    pinocchio::cpi::invoke(
        &instruction,
        &[payer, token_account, owner, mint, system_program],
    )
}

/// Create associated token account if it doesn't exist (idempotent)
/// Uses ATA program's CreateIdempotent instruction
#[inline]
pub fn create_associated_token_account_idempotent<'a>(
    payer: &'a AccountView,
    owner: &'a AccountView,
    mint: &'a AccountView,
    token_account: &'a AccountView,
    system_program: &'a AccountView,
    ata_program: &'a AccountView,
) -> Result<(), ProgramError> {
    // SAFETY: We only read the owner field to identify the token program.
    let token_program = unsafe { mint.owner() };

    // ATA CreateIdempotent instruction = 1
    let instruction_data = [1u8];

    let account_metas = [
        pinocchio::instruction::InstructionAccount {
            address: payer.address(),
            is_writable: true,
            is_signer: true,
        },
        pinocchio::instruction::InstructionAccount {
            address: token_account.address(),
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
            address: token_program,
            is_writable: false,
            is_signer: false,
        },
    ];

    let instruction = pinocchio::instruction::InstructionView {
        program_id: ata_program.address(),
        accounts: &account_metas,
        data: &instruction_data,
    };

    // For CPI, only pass accounts that are in account_metas
    pinocchio::cpi::invoke(
        &instruction,
        &[payer, token_account, owner, mint, system_program],
    )
}

/// Derive associated token account address for SPL Token
#[inline]
pub fn get_associated_token_address(owner: &Address, mint: &Address) -> Address {
    let seeds: &[&[u8]] = &[owner.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()];
    let (address, _bump) = Address::find_program_address(seeds, &ASSOCIATED_TOKEN_PROGRAM_ID);
    address
}

/// Derive associated token account address for Token-2022
#[inline]
pub fn get_associated_token_address_22(owner: &Address, mint: &Address) -> Address {
    let seeds: &[&[u8]] = &[
        owner.as_ref(),
        TOKEN_2022_PROGRAM_ID.as_ref(),
        mint.as_ref(),
    ];
    let (address, _bump) = Address::find_program_address(seeds, &ASSOCIATED_TOKEN_PROGRAM_ID);
    address
}

/// Derive associated token account address for PToken
#[inline]
pub fn get_associated_token_address_ptoken(owner: &Address, mint: &Address) -> Address {
    let seeds: &[&[u8]] = &[
        owner.as_ref(),
        PTOKEN_PROGRAM_ID.as_ref(),
        mint.as_ref(),
    ];
    let (address, _bump) = Address::find_program_address(seeds, &ASSOCIATED_TOKEN_PROGRAM_ID);
    address
}

// ============================================================================
// Mint/Burn Operations
// ============================================================================

/// Mint tokens to an account (requires mint authority)
///
/// Pass `&[]` for `signer_seeds` if the authority is a regular signer (not a PDA).
#[inline]
pub fn mint_to<'a>(
    mint: &'a AccountView,
    destination: &'a AccountView,
    authority: &'a AccountView,
    amount: u64,
    signer_seeds: &[Seed<'a>],
) -> Result<(), ProgramError> {
    // MintTo instruction = 7
    let mut instruction_data = [0u8; 9];
    instruction_data[0] = 7;
    instruction_data[1..9].copy_from_slice(&amount.to_le_bytes());

    let account_metas = [
        pinocchio::instruction::InstructionAccount {
            address: mint.address(),
            is_writable: true,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: destination.address(),
            is_writable: true,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: authority.address(),
            is_writable: false,
            is_signer: true,
        },
    ];

    // SAFETY: We only read the owner field to identify the token program.
    let token_program = unsafe { mint.owner() };

    let instruction = pinocchio::instruction::InstructionView {
        program_id: token_program,
        accounts: &account_metas,
        data: &instruction_data,
    };

    invoke_maybe_signed(&instruction, &[mint, destination, authority], signer_seeds)
}

/// Initialize a new mint account using InitializeMint2 (no rent sysvar required)
/// Account must already be allocated with MINT_SIZE bytes and owned by token program
#[inline]
pub fn initialize_mint(
    mint: &AccountView,
    mint_authority: &Address,
    freeze_authority: Option<&Address>,
    decimals: u8,
) -> Result<(), ProgramError> {
    // InitializeMint2 instruction = 20 (doesn't require Rent sysvar)
    // Data: [20, decimals, mint_authority(32), freeze_authority_option(1), freeze_authority(32)?]
    let mut instruction_data = [0u8; 67]; // 1 + 1 + 32 + 1 + 32
    instruction_data[0] = 20; // InitializeMint2
    instruction_data[1] = decimals;
    instruction_data[2..34].copy_from_slice(mint_authority.as_ref());

    let data_len = if let Some(freeze_auth) = freeze_authority {
        instruction_data[34] = 1; // Some
        instruction_data[35..67].copy_from_slice(freeze_auth.as_ref());
        67
    } else {
        instruction_data[34] = 0; // None
        35
    };

    let account_metas = [pinocchio::instruction::InstructionAccount {
        address: mint.address(),
        is_writable: true,
        is_signer: false,
    }];

    // SAFETY: We only read the owner field to identify the token program.
    let token_program = unsafe { mint.owner() };

    let instruction = pinocchio::instruction::InstructionView {
        program_id: token_program,
        accounts: &account_metas,
        data: &instruction_data[..data_len],
    };

    // For CPI, only pass accounts that are in account_metas
    // The program is determined by instruction.program_id
    pinocchio::cpi::invoke(&instruction, &[mint])
}

/// Burn tokens from an account (requires owner/delegate authority)
///
/// Pass `&[]` for `signer_seeds` if the authority is a regular signer (not a PDA).
#[inline]
pub fn burn<'a>(
    account: &'a AccountView,
    mint: &'a AccountView,
    authority: &'a AccountView,
    amount: u64,
    signer_seeds: &[Seed<'a>],
) -> Result<(), ProgramError> {
    // Burn instruction = 8
    let mut instruction_data = [0u8; 9];
    instruction_data[0] = 8;
    instruction_data[1..9].copy_from_slice(&amount.to_le_bytes());

    let account_metas = [
        pinocchio::instruction::InstructionAccount {
            address: account.address(),
            is_writable: true,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: mint.address(),
            is_writable: true,
            is_signer: false,
        },
        pinocchio::instruction::InstructionAccount {
            address: authority.address(),
            is_writable: false,
            is_signer: true,
        },
    ];

    // SAFETY: We only read the owner field to identify the token program.
    let token_program = unsafe { account.owner() };

    let instruction = pinocchio::instruction::InstructionView {
        program_id: token_program,
        accounts: &account_metas,
        data: &instruction_data,
    };

    invoke_maybe_signed(&instruction, &[account, mint, authority], signer_seeds)
}
