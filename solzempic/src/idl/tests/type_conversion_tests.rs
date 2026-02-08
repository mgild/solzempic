//! Tests for Rust type to IDL type conversion

use crate::idl::rust_type_to_idl_json;
use alloc::vec;

#[test]
fn test_primitive_u8() {
    assert_eq!(rust_type_to_idl_json("u8"), "\"u8\"");
}

#[test]
fn test_primitive_u16() {
    assert_eq!(rust_type_to_idl_json("u16"), "\"u16\"");
}

#[test]
fn test_primitive_u32() {
    assert_eq!(rust_type_to_idl_json("u32"), "\"u32\"");
}

#[test]
fn test_primitive_u64() {
    assert_eq!(rust_type_to_idl_json("u64"), "\"u64\"");
}

#[test]
fn test_primitive_u128() {
    assert_eq!(rust_type_to_idl_json("u128"), "\"u128\"");
}

#[test]
fn test_primitive_i8() {
    assert_eq!(rust_type_to_idl_json("i8"), "\"i8\"");
}

#[test]
fn test_primitive_i16() {
    assert_eq!(rust_type_to_idl_json("i16"), "\"i16\"");
}

#[test]
fn test_primitive_i32() {
    assert_eq!(rust_type_to_idl_json("i32"), "\"i32\"");
}

#[test]
fn test_primitive_i64() {
    assert_eq!(rust_type_to_idl_json("i64"), "\"i64\"");
}

#[test]
fn test_primitive_i128() {
    assert_eq!(rust_type_to_idl_json("i128"), "\"i128\"");
}

#[test]
fn test_primitive_bool() {
    assert_eq!(rust_type_to_idl_json("bool"), "\"bool\"");
}

#[test]
fn test_primitive_f32() {
    assert_eq!(rust_type_to_idl_json("f32"), "\"f32\"");
}

#[test]
fn test_primitive_f64() {
    assert_eq!(rust_type_to_idl_json("f64"), "\"f64\"");
}

#[test]
fn test_pubkey() {
    assert_eq!(rust_type_to_idl_json("Pubkey"), "\"pubkey\"");
}

#[test]
fn test_address() {
    assert_eq!(rust_type_to_idl_json("Address"), "\"pubkey\"");
}

#[test]
fn test_fully_qualified_address() {
    assert_eq!(
        rust_type_to_idl_json("solana_address::Address"),
        "\"pubkey\""
    );
}

#[test]
fn test_fixed_array_u8() {
    assert_eq!(rust_type_to_idl_json("[u8; 32]"), "{ \"array\": [\"u8\", 32] }");
}

#[test]
fn test_fixed_array_u64() {
    assert_eq!(
        rust_type_to_idl_json("[u64; 4]"),
        "{ \"array\": [\"u64\", 4] }"
    );
}

#[test]
fn test_fixed_array_pubkey() {
    assert_eq!(
        rust_type_to_idl_json("[Pubkey; 2]"),
        "{ \"array\": [\"pubkey\", 2] }"
    );
}

#[test]
fn test_array_single_element() {
    assert_eq!(rust_type_to_idl_json("[u8; 1]"), "{ \"array\": [\"u8\", 1] }");
}

#[test]
fn test_array_large_size() {
    assert_eq!(
        rust_type_to_idl_json("[u8; 1024]"),
        "{ \"array\": [\"u8\", 1024] }"
    );
}

#[test]
fn test_custom_type() {
    assert_eq!(rust_type_to_idl_json("MyCustomType"), "\"MyCustomType\"");
}

#[test]
fn test_nested_module_type() {
    assert_eq!(rust_type_to_idl_json("module::Type"), "\"module::Type\"");
}

#[test]
fn test_array_with_const_expr() {
    // Arrays with const expressions should fall back to "bytes"
    assert_eq!(rust_type_to_idl_json("[u8; SIZE]"), "\"bytes\"");
}

#[test]
fn test_all_primitive_types() {
    let primitives = vec![
        ("u8", "\"u8\""),
        ("u16", "\"u16\""),
        ("u32", "\"u32\""),
        ("u64", "\"u64\""),
        ("u128", "\"u128\""),
        ("i8", "\"i8\""),
        ("i16", "\"i16\""),
        ("i32", "\"i32\""),
        ("i64", "\"i64\""),
        ("i128", "\"i128\""),
        ("f32", "\"f32\""),
        ("f64", "\"f64\""),
        ("bool", "\"bool\""),
    ];

    for (rust_type, expected) in primitives {
        assert_eq!(rust_type_to_idl_json(rust_type), expected);
    }
}

#[test]
fn test_various_array_sizes() {
    let arrays = vec![
        ("[u8; 1]", "{ \"array\": [\"u8\", 1] }"),
        ("[u8; 8]", "{ \"array\": [\"u8\", 8] }"),
        ("[u8; 16]", "{ \"array\": [\"u8\", 16] }"),
        ("[u8; 32]", "{ \"array\": [\"u8\", 32] }"),
        ("[u8; 64]", "{ \"array\": [\"u8\", 64] }"),
        ("[u8; 128]", "{ \"array\": [\"u8\", 128] }"),
        ("[u8; 256]", "{ \"array\": [\"u8\", 256] }"),
    ];

    for (rust_type, expected) in arrays {
        assert_eq!(rust_type_to_idl_json(rust_type), expected);
    }
}
