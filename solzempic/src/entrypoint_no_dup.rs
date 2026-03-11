//! No-duplicate-accounts entrypoint for maximum compute efficiency.
//!
//! Skips the `borrow_state == NON_DUP_MARKER` branch that pinocchio checks per
//! account, saving ~3-4 CU per account. For a 4-account instruction: ~12-16 CU.
//!
//! # Safety contract
//!
//! The caller MUST guarantee that **no duplicate accounts** are ever passed to
//! this program. Passing duplicate account slots causes undefined behavior
//! (misinterpreted dup stubs as full account pointers → out-of-bounds reads,
//! data corruption, silent aliasing of mutable state).

use core::{
    cmp::min,
    mem::{size_of, MaybeUninit},
    ptr::with_exposed_provenance_mut,
    slice::from_raw_parts,
};

use pinocchio::{
    account::{AccountView, RuntimeAccount, MAX_PERMITTED_DATA_INCREASE},
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

/// Process one account (no dup check) — mirrors pinocchio's `process_n_accounts!(@process_account)`.
macro_rules! process_account_no_dup {
    ($input:ident, $accounts:ident) => {
        $accounts = $accounts.add(1);
        let account: *mut RuntimeAccount = $input as *mut RuntimeAccount;
        $input = $input.add(size_of::<u64>());
        $accounts.write(AccountView::new_unchecked(account));
        advance_input_with_account!($input, account);
    };
}

/// Process N accounts at once (unrolled) — mirrors pinocchio's `process_accounts!`.
macro_rules! process_accounts_no_dup {
    (1 => ($input:ident, $accounts:ident)) => {
        process_account_no_dup!($input, $accounts);
    };
    (2 => ($input:ident, $accounts:ident)) => {
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
    };
    (3 => ($input:ident, $accounts:ident)) => {
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
    };
    (4 => ($input:ident, $accounts:ident)) => {
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
    };
    (5 => ($input:ident, $accounts:ident)) => {
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
        process_account_no_dup!($input, $accounts);
    };
}

/// Parse SVM input buffer into `accounts`, skipping the per-account dup check.
///
/// Mirrors pinocchio's `deserialize` exactly except the `borrow_state == NON_DUP_MARKER`
/// branch is removed — every account slot is always treated as non-duplicate.
///
/// # Safety
///
/// - `input` must be a valid SVM serialized input buffer.
/// - All account slots must be non-duplicate (`borrow_state == 0xFF`).
///   Duplicate slots cause undefined behavior.
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
        let mut accounts = accounts.as_mut_ptr() as *mut AccountView;

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

            // Unrolled like pinocchio for minimum branching CU.
            if to_process_plus_one == 2 {
                process_accounts_no_dup!(1 => (input, accounts));
            } else {
                while to_process_plus_one > 5 {
                    process_accounts_no_dup!(5 => (input, accounts));
                    to_process_plus_one -= 5;
                }
                match to_process_plus_one {
                    5 => {
                        process_accounts_no_dup!(4 => (input, accounts));
                    }
                    4 => {
                        process_accounts_no_dup!(3 => (input, accounts));
                    }
                    3 => {
                        process_accounts_no_dup!(2 => (input, accounts));
                    }
                    2 => {
                        process_accounts_no_dup!(1 => (input, accounts));
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
                    // No dup guarantee: always advance as non-dup.
                    advance_input_with_account!(input, account);
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
/// # Safety
///
/// - `input` must point to the first serialized account in a valid SVM input buffer.
/// - All account slots must be non-duplicate (`borrow_state == 0xFF`).
/// - Exactly `N` non-duplicate accounts must follow at `input`.
#[inline(always)]
pub unsafe fn deserialize_no_dup_ptrs<const N: usize>(
    mut input: *mut u8,
) -> ([*mut RuntimeAccount; N], &'static Address, &'static [u8]) {
    // Note: input already points to the first account — no skip needed here.

    let mut ptrs = [core::ptr::null_mut::<RuntimeAccount>(); N];
    let mut i = 0;
    while i < N {
        let account: *mut RuntimeAccount = input as *mut RuntimeAccount;
        ptrs[i] = account;
        input = input.add(size_of::<u64>());
        advance_input_with_account!(input, account);
        i += 1;
    }

    // Instruction data.
    let instruction_data_len = *(input as *const u64) as usize;
    input = input.add(size_of::<u64>());
    let instruction_data = from_raw_parts(input, instruction_data_len);
    let input = input.add(instruction_data_len);

    // Program ID.
    let program_id: &'static Address = &*(input as *const Address);

    (ptrs, program_id, instruction_data)
}

/// Program entrypoint — no dup check variant.
///
/// Drop-in for pinocchio's `process_entrypoint`. Parses accounts without the
/// per-account `borrow_state == NON_DUP_MARKER` branch.
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
