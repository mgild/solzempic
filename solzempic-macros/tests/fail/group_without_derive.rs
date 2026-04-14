//! Fail case: `#[group]` field references a type that does NOT
//! `#[derive(AccountGroupFields)]` and does NOT `impl AccountGroup<'_>`.
//!
//! Expected: the compiler error points at the named const
//! `__SOLZEMPIC_GROUP_CHECK_DoThing_trader_ctx` emitted by the macro's
//! inline trait-bound assertion — much more actionable than a raw error
//! buried inside a deeply nested const expression.
//!
//! The stderr check is tolerant (`compile_fail` mode), so minor rustc
//! formatting drift won't break the test; we only need the key phrase
//! `AccountGroup` and our named check const to appear.

use solzempic::{instruction, Signer};

// A plain struct with NO derive and NO AccountGroup impl.
pub struct NotAGroup<'a> {
    _phantom: core::marker::PhantomData<&'a ()>,
}

#[instruction]
pub struct DoThing<'a> {
    pub payer: Signer<'a>,
    #[group]
    pub trader_ctx: NotAGroup<'a>,
}

fn main() {}
