//! Pass case: `AccountGroupFields` on a struct mixing a single-slot
//! `AccountRefMut` with a `ShardListRefMut` field (which expands to 2 slots
//! named `<field>_current` and `<field>_next`). Total slots = 3.

use bytemuck::{Pod, Zeroable};
use solana_address::Address;
use solzempic::{AccountGroupField, Loadable, ShardListNode};

pub struct TestFramework;
impl solzempic::Framework for TestFramework {
    const PROGRAM_ID: Address = Address::new_from_array([0u8; 32]);
}

pub type AccountRefMut<'a, T> = solzempic::AccountRefMut<'a, T, TestFramework>;
pub type ShardListRefMut<'a, T> = solzempic::ShardListRefMut<'a, T, TestFramework>;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Market {
    pub discriminator: [u8; 8],
    pub _pad: [u8; 24],
}

impl Loadable for Market {
    const DISCRIMINATOR: u8 = 1;
    const LEN: usize = core::mem::size_of::<Self>();
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct OrderShard {
    pub discriminator: [u8; 8],
    pub next_shard: Address,
}

impl Loadable for OrderShard {
    const DISCRIMINATOR: u8 = 2;
    const LEN: usize = core::mem::size_of::<Self>();
}

impl ShardListNode for OrderShard {
    fn next_shard(&self) -> &Address {
        &self.next_shard
    }
    fn next_shard_mut(&mut self) -> &mut Address {
        &mut self.next_shard
    }
}

#[derive(solzempic::AccountGroupFields)]
pub struct MixedCtx<'a> {
    pub market: AccountRefMut<'a, Market>,
    pub shards: ShardListRefMut<'a, OrderShard>,
}

fn main() {
    assert_eq!(MixedCtx::DERIVED_FIELD_COUNT, 3);
    let meta: &'static [AccountGroupField] = MixedCtx::DERIVED_FIELD_METADATA;
    assert_eq!(meta.len(), 3);

    assert_eq!(meta[0].name, "market");
    assert!(meta[0].is_writable);

    assert_eq!(meta[1].name, "shards_current");
    assert!(meta[1].is_writable);
    assert_eq!(meta[2].name, "shards_next");
    assert!(meta[2].is_writable);
}
