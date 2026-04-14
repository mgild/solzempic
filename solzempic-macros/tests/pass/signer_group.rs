//! Pass case: `AccountGroupFields` tags the `Signer` field with
//! `is_signer = true` and the `AccountRefMut` field with `is_writable = true`
//! (per-field attribute inference via `analyze_field_type`).

use bytemuck::{Pod, Zeroable};
use solana_address::Address;
use solzempic::{AccountGroupField, Loadable, Signer};

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

#[derive(solzempic::AccountGroupFields)]
pub struct TraderAuthCtx<'a> {
    pub owner: Signer<'a>,
    pub trader: AccountRefMut<'a, Trader>,
}

fn main() {
    assert_eq!(TraderAuthCtx::DERIVED_FIELD_COUNT, 2);
    let meta: &'static [AccountGroupField] = TraderAuthCtx::DERIVED_FIELD_METADATA;
    assert_eq!(meta.len(), 2);

    // Signer field: is_signer = true, is_writable = false.
    assert_eq!(meta[0].name, "owner");
    assert!(meta[0].is_signer);
    assert!(!meta[0].is_writable);
    assert!(!meta[0].is_program);

    // AccountRefMut field: is_writable = true, is_signer = false.
    assert_eq!(meta[1].name, "trader");
    assert!(!meta[1].is_signer);
    assert!(meta[1].is_writable);
    assert!(!meta[1].is_program);
}
