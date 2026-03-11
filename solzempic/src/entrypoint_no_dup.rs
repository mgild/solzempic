//! Optimized entrypoint with optional duplicate-account handling.
//!
//! The [`deserialize_no_dup_ptrs`] and [`deserialize_no_dup`] functions now
//! correctly handle SBF duplicate-account stubs at the cost of ~2 CU per
//! account (one byte load + one compare/branch per slot). For a 4-account
//! instruction with no dups the typical overhead is ~8 CU over the former
//! unconditional-non-dup variant.
//!
//! A dup stub is 8 bytes: `[dup_index: u8, 0, 0, 0, 0, 0, 0, 0]`. When the
//! first byte (`borrow_state`) is not `NON_DUP_MARKER` (0xFF), it is the
//! index of the earlier account whose pointer should be reused.

use core::{
    cmp::min,
    mem::{size_of, MaybeUninit},
    ptr::with_exposed_provenance_mut,
    slice::from_raw_parts,
};

use pinocchio::{
    account::{AccountView, RuntimeAccount, MAX_PERMITTED_DATA_INCREASE},
    entrypoint::NON_DUP_MARKER,
    Address, ProgramResult, MAX_TX_ACCOUNTS, SUCCESS,
};

/// `size_of::<RuntimeAccount>()` + `MAX_PERMITTED_DATA_INCREASE`.
const STATIC_ACCOUNT_DATA: usize = size_of::<RuntimeAccount>() + MAX_PERMITTED_DATA_INCREASE;

/// BPF alignment of u128 (8 bytes on SBF).
const BPF_ALIGN_OF_U128: usize = 8;

/// Align `input` to `BPF_ALIGN_OF_U128` — mirrors pinocchio's `align_pointer!`.
macro_rules! align_pointer {
    ($ptr:ident) => {
        with_exposed_provenance_mut(
            ($ptr.expose_provenance() + (BPF_ALIGN_OF_U128 - 1)) & !(BPF_ALIGN_OF_U128 - 1),
        )
    };
}

/// Advance `$input` past one non-dup account — mirrors pinocchio's `advance_input_with_account!`.
macro_rules! advance_input_with_account {
    ($input:ident, $account:expr) => {{
        $input = $input.add(STATIC_ACCOUNT_DATA);
        $input = $input.add((*$account).data_len as usize);
        $input = align_pointer!($input);
    }};
}

/// Process one account with dup handling.
///
/// `$accounts_base` points to `accounts[0]` and is used to copy an earlier
/// `AccountView` when a dup stub is encountered.
macro_rules! process_account {
    ($input:ident, $accounts:ident, $accounts_base:ident) => {
        $accounts = $accounts.add(1);
        let account: *mut RuntimeAccount = $input as *mut RuntimeAccount;
        $input = $input.add(size_of::<u64>());
        if (*account).borrow_state != NON_DUP_MARKER {
            // Dup stub: copy the AccountView already written at the referenced index.
            $accounts.write(*$accounts_base.add((*account).borrow_state as usize));
        } else {
            $accounts.write(AccountView::new_unchecked(account));
            advance_input_with_account!($input, account);
        }
    };
}

/// Process N accounts at once (unrolled).
macro_rules! process_accounts {
    (1 => ($input:ident, $accounts:ident, $accounts_base:ident)) => {
        process_account!($input, $accounts, $accounts_base);
    };
    (2 => ($input:ident, $accounts:ident, $accounts_base:ident)) => {
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
    };
    (3 => ($input:ident, $accounts:ident, $accounts_base:ident)) => {
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
    };
    (4 => ($input:ident, $accounts:ident, $accounts_base:ident)) => {
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
    };
    (5 => ($input:ident, $accounts:ident, $accounts_base:ident)) => {
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
        process_account!($input, $accounts, $accounts_base);
    };
}

/// Parse SVM input buffer into `accounts`, handling duplicate-account stubs.
///
/// Mirrors pinocchio's `deserialize` but replaces the full dup-handling path
/// with the lighter-weight approach from this module: one byte load + branch
/// per account instead of a slice-based clone.
///
/// # Safety
///
/// - `input` must be a valid SVM serialized input buffer.
/// - Dup indices must reference a previously-seen account (`< current index`).
#[inline(always)]
pub unsafe fn deserialize_no_dup<const MAX_ACCOUNTS: usize>(
    mut input: *mut u8,
    accounts: &mut [MaybeUninit<AccountView>; MAX_ACCOUNTS],
) -> (&'static Address, usize, &'static [u8]) {
    const {
        assert!(
            MAX_ACCOUNTS <= MAX_TX_ACCOUNTS,
            "MAX_ACCOUNTS must be <= MAX_TX_ACCOUNTS (255)",
        );
    }

    let mut processed = *(input as *const u64) as usize;
    input = input.add(size_of::<u64>());

    if processed > 0 {
        let accounts_base = accounts.as_mut_ptr() as *mut AccountView;
        let mut accounts = accounts_base;

        // First account is always non-dup (SVM guarantee).
        let account: *mut RuntimeAccount = input as *mut RuntimeAccount;
        accounts.write(AccountView::new_unchecked(account));
        input = input.add(size_of::<u64>());
        advance_input_with_account!(input, account);

        if processed > 1 {
            let mut to_process_plus_one = if MAX_ACCOUNTS < MAX_TX_ACCOUNTS {
                min(processed, MAX_ACCOUNTS)
            } else {
                processed
            };

            let mut to_skip = processed - to_process_plus_one;
            processed = to_process_plus_one;

            // Unrolled for minimum branching CU.
            if to_process_plus_one == 2 {
                process_accounts!(1 => (input, accounts, accounts_base));
            } else {
                while to_process_plus_one > 5 {
                    process_accounts!(5 => (input, accounts, accounts_base));
                    to_process_plus_one -= 5;
                }
                match to_process_plus_one {
                    5 => {
                        process_accounts!(4 => (input, accounts, accounts_base));
                    }
                    4 => {
                        process_accounts!(3 => (input, accounts, accounts_base));
                    }
                    3 => {
                        process_accounts!(2 => (input, accounts, accounts_base));
                    }
                    2 => {
                        process_accounts!(1 => (input, accounts, accounts_base));
                    }
                    1 => (),
                    _ => unsafe { core::hint::unreachable_unchecked() },
                }
            }

            // Skip accounts beyond MAX_ACCOUNTS to reach instruction data.
            // Only possible when MAX_ACCOUNTS < MAX_TX_ACCOUNTS.
            if MAX_ACCOUNTS < MAX_TX_ACCOUNTS {
                while to_skip > 0 {
                    to_skip -= 1;
                    let account: *mut RuntimeAccount = input as *mut RuntimeAccount;
                    input = input.add(size_of::<u64>());
                    if (*account).borrow_state == NON_DUP_MARKER {
                        advance_input_with_account!(input, account);
                    }
                }
            }
        }
    }

    // Instruction data.
    let instruction_data_len = *(input as *const u64) as usize;
    input = input.add(size_of::<u64>());
    let instruction_data = from_raw_parts(input, instruction_data_len);
    let input = input.add(instruction_data_len);

    // Program ID.
    let program_id: &'static Address = &*(input as *const Address);

    (program_id, processed, instruction_data)
}

/// Parse N accounts from the SBF input buffer into raw `RuntimeAccount` pointers.
///
/// Unlike [`deserialize_no_dup`], this returns raw pointers directly into the SBF input buffer
/// rather than `AccountView` wrappers, and does not require an output array parameter.
///
/// **`input` must point to the first account** (i.e. past the leading `num_accounts: u64`).
/// The caller is responsible for advancing past that field before calling this function.
/// This avoids a redundant pointer advance when the caller already read `num_accounts`.
///
/// Returns `Err(InvalidAccountData)` if any account slot is a duplicate stub
/// (`borrow_state != NON_DUP_MARKER`). One byte checked per account (~N CU).
///
/// # Safety
///
/// - `input` must point to the first serialized account in a valid SVM input buffer.
/// - Exactly `N` accounts must follow at `input`.
#[inline(always)]
pub unsafe fn deserialize_no_dup_ptrs<const N: usize>(
    mut input: *mut u8,
) -> Result<([*mut RuntimeAccount; N], &'static Address, &'static [u8]), pinocchio::error::ProgramError>
{
    // Note: input already points to the first account — no skip needed here.

    let mut ptrs = [core::ptr::null_mut::<RuntimeAccount>(); N];
    let mut i = 0;
    while i < N {
        let account: *mut RuntimeAccount = input as *mut RuntimeAccount;
        if (*account).borrow_state != NON_DUP_MARKER {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        ptrs[i] = account;
        input = input.add(size_of::<u64>());
        advance_input_with_account!(input, account);
        i += 1;
    }

    // Instruction data.
    let instruction_data_len = *(input as *const u64) as usize;
    input = input.add(size_of::<u64>());
    let instruction_data: &'static [u8] = from_raw_parts(input, instruction_data_len);
    let input = input.add(instruction_data_len);

    // Program ID.
    let program_id: &'static Address = &*(input as *const Address);

    Ok((ptrs, program_id, instruction_data))
}

/// Program entrypoint — optimized variant with dup-account handling.
///
/// Drop-in for pinocchio's `process_entrypoint`. Handles SBF duplicate-account
/// stubs (~2 CU per account overhead vs. a pure no-dup path) while being
/// meaningfully cheaper than pinocchio's full `clone_account_view` path.
#[inline(always)]
pub unsafe fn process_entrypoint_no_dup<const MAX_ACCOUNTS: usize>(
    input: *mut u8,
    process_instruction: fn(&Address, &[AccountView], &[u8]) -> ProgramResult,
) -> u64 {
    const UNINIT: MaybeUninit<AccountView> = MaybeUninit::<AccountView>::uninit();
    let mut accounts = [UNINIT; MAX_ACCOUNTS];

    let (program_id, count, instruction_data) =
        unsafe { deserialize_no_dup::<MAX_ACCOUNTS>(input, &mut accounts) };

    match process_instruction(
        program_id,
        unsafe { from_raw_parts(accounts.as_ptr() as _, count) },
        instruction_data,
    ) {
        Ok(()) => SUCCESS,
        Err(error) => error.into(),
    }
}
