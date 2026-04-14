//! Pass case: `#[instruction]` with a `#[group]` field referencing a type
//! that derives `AccountGroupFields` and implements `AccountGroup`.
//!
//! Verifies that `SHANK_ACCOUNTS` concatenates the group's fields in order
//! and that `NUM_ACCOUNTS` sums correctly across inlined + group fields.

use bytemuck::{Pod, Zeroable};
use pinocchio::{error::ProgramError, AccountView};
use solana_address::Address;
use solzempic::{instruction, AccountGroup, AccountGroupField, Loadable, Signer, ValidatedAccount};

pub struct TestFramework;
impl solzempic::Framework for TestFramework {
    const PROGRAM_ID: Address = Address::new_from_array([0u8; 32]);
}

pub type AccountRefMut<'a, T> = solzempic::AccountRefMut<'a, T, TestFramework>;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Trader {
    pub discriminator: [u8; 8],
    pub owner: Address,
}

impl Loadable for Trader {
    const DISCRIMINATOR: u8 = 1;
    const LEN: usize = core::mem::size_of::<Self>();
}

// Group definition — provides metadata for two accounts (owner + trader).
#[derive(solzempic::AccountGroupFields)]
pub struct TraderCtx<'a> {
    pub owner: Signer<'a>,
    pub trader: AccountRefMut<'a, Trader>,
}

impl<'a> AccountGroup<'a> for TraderCtx<'a> {
    const ACCOUNT_COUNT: usize = <Self>::DERIVED_FIELD_COUNT;
    const FIELD_METADATA: &'static [AccountGroupField] = <Self>::DERIVED_FIELD_METADATA;

    fn load(accounts: &'a [AccountView]) -> Result<Self, ProgramError> {
        let mut it = accounts.iter();
        let mut one = || it.next().ok_or(ProgramError::NotEnoughAccountKeys);
        Ok(Self {
            owner: Signer::wrap(one()?)?,
            trader: AccountRefMut::load(one()?)?,
        })
    }
}

// Instruction struct using #[group] — the macro expands SHANK_ACCOUNTS from
// `TraderCtx::FIELD_METADATA` at const-eval time.
#[instruction]
pub struct DoThing<'a> {
    pub payer: Signer<'a>,
    #[group]
    pub trader_ctx: TraderCtx<'a>,
}

fn main() {
    // 1 (payer) + 2 (trader_ctx) = 3 slots.
    assert_eq!(DoThing::NUM_ACCOUNTS, 3);

    let arr = DoThing::SHANK_ACCOUNTS;
    assert_eq!(arr.len(), 3);

    // Slot 0: payer (signer).
    assert_eq!(arr[0].index, 0);
    assert_eq!(arr[0].name, "payer");
    assert!(arr[0].is_signer);

    // Slot 1: owner (from TraderCtx) — signer.
    assert_eq!(arr[1].index, 1);
    assert_eq!(arr[1].name, "owner");
    assert!(arr[1].is_signer);

    // Slot 2: trader (from TraderCtx) — writable.
    assert_eq!(arr[2].index, 2);
    assert_eq!(arr[2].name, "trader");
    assert!(arr[2].is_writable);
}
