//! Fail case: `#[derive(AccountGroupFields)]` applied to a type it doesn't
//! support (here: a non-struct / tuple struct) should produce an actionable
//! panic message from the proc macro rather than a confusing unrelated error.
//!
//! The proc macro panic surfaces as a compile error at the call site; the
//! `compile_fail` matcher is tolerant to panic-message formatting drift.

// Tuple structs are unsupported — the derive expects named fields.
#[derive(solzempic::AccountGroupFields)]
pub struct TupleCtx<'a>(pub core::marker::PhantomData<&'a ()>);

fn main() {}
