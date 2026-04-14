//! trybuild test harness for `solzempic-macros`.
//!
//! These tests invoke the compiler on each fixture file as a separate crate
//! and verify either successful compilation (pass cases) or that the compiler
//! produces an expected error diagnostic (fail cases).
//!
//! Fail-case `.stderr` files are tolerant of minor rustc version drift — to
//! regenerate them after an intentional macro-diagnostic change, run:
//!
//! ```text
//! TRYBUILD=overwrite cargo test -p solzempic-macros --test trybuild
//! ```

#[test]
fn trybuild_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/pass/simple_group.rs");
    t.pass("tests/pass/mixed_group.rs");
    t.pass("tests/pass/signer_group.rs");
    t.pass("tests/pass/instruction_with_group.rs");
}

#[test]
fn trybuild_fail() {
    let t = trybuild::TestCases::new();
    // `compile_fail` is more tolerant than `pass` across rustc versions: the
    // stderr snapshot only has to contain the expected marker lines, not
    // match byte-for-byte. This keeps the suite green when rustc tweaks its
    // trait-bound error formatting.
    t.compile_fail("tests/fail/group_without_derive.rs");
    t.compile_fail("tests/fail/unknown_multi_type.rs");
}
