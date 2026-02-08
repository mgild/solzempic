//! Tests for event metadata and IDL generation

use crate::event::{EventFieldMeta, EventMeta};
use alloc::format;

#[test]
fn test_event_field_meta_creation() {
    let meta = EventFieldMeta {
        name: "amount",
        type_name: "u64",
    };

    assert_eq!(meta.name, "amount");
    assert_eq!(meta.type_name, "u64");
}

#[test]
fn test_event_field_meta_various_types() {
    let primitives = [
        ("value_u8", "u8"),
        ("value_u16", "u16"),
        ("value_u32", "u32"),
        ("value_u64", "u64"),
        ("value_i8", "i8"),
        ("value_i16", "i16"),
        ("value_i32", "i32"),
        ("value_i64", "i64"),
        ("flag", "bool"),
    ];

    for (name, type_name) in primitives {
        let meta = EventFieldMeta { name, type_name };
        assert_eq!(meta.name, name);
        assert_eq!(meta.type_name, type_name);
    }
}

#[test]
fn test_event_field_meta_solana_types() {
    let meta_address = EventFieldMeta {
        name: "authority",
        type_name: "Address",
    };
    assert_eq!(meta_address.type_name, "Address");

    let meta_pubkey = EventFieldMeta {
        name: "account",
        type_name: "Pubkey",
    };
    assert_eq!(meta_pubkey.type_name, "Pubkey");
}

#[test]
fn test_event_field_meta_arrays() {
    let meta = EventFieldMeta {
        name: "data",
        type_name: "[u8; 32]",
    };
    assert_eq!(meta.name, "data");
    assert_eq!(meta.type_name, "[u8; 32]");
}

#[test]
fn test_event_meta_creation() {
    static FIELDS: &[EventFieldMeta] = &[
        EventFieldMeta {
            name: "from",
            type_name: "Address",
        },
        EventFieldMeta {
            name: "to",
            type_name: "Address",
        },
        EventFieldMeta {
            name: "amount",
            type_name: "u64",
        },
    ];

    let meta = EventMeta {
        name: "TransferEvent",
        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
        fields: FIELDS,
    };

    assert_eq!(meta.name, "TransferEvent");
    assert_eq!(meta.discriminator, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(meta.fields.len(), 3);
}

#[test]
fn test_event_meta_field_access() {
    static FIELDS: &[EventFieldMeta] = &[
        EventFieldMeta {
            name: "value",
            type_name: "u64",
        },
        EventFieldMeta {
            name: "flag",
            type_name: "u8",
        },
    ];

    let meta = EventMeta {
        name: "TestEvent",
        discriminator: [10, 20, 30, 40, 50, 60, 70, 80],
        fields: FIELDS,
    };

    assert_eq!(meta.fields[0].name, "value");
    assert_eq!(meta.fields[0].type_name, "u64");
    assert_eq!(meta.fields[1].name, "flag");
    assert_eq!(meta.fields[1].type_name, "u8");
}

#[test]
fn test_event_meta_empty_fields() {
    static FIELDS: &[EventFieldMeta] = &[];

    let meta = EventMeta {
        name: "EmptyEvent",
        discriminator: [0, 0, 0, 0, 0, 0, 0, 0],
        fields: FIELDS,
    };

    assert_eq!(meta.fields.len(), 0);
}

#[test]
fn test_event_meta_single_field() {
    static FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "timestamp",
        type_name: "i64",
    }];

    let meta = EventMeta {
        name: "TimestampEvent",
        discriminator: [99, 99, 99, 99, 99, 99, 99, 99],
        fields: FIELDS,
    };

    assert_eq!(meta.fields.len(), 1);
    assert_eq!(meta.fields[0].name, "timestamp");
}

#[test]
fn test_event_meta_many_fields() {
    static FIELDS: &[EventFieldMeta] = &[
        EventFieldMeta {
            name: "field1",
            type_name: "u8",
        },
        EventFieldMeta {
            name: "field2",
            type_name: "u16",
        },
        EventFieldMeta {
            name: "field3",
            type_name: "u32",
        },
        EventFieldMeta {
            name: "field4",
            type_name: "u64",
        },
        EventFieldMeta {
            name: "field5",
            type_name: "Address",
        },
    ];

    let meta = EventMeta {
        name: "ComplexEvent",
        discriminator: [88, 88, 88, 88, 88, 88, 88, 88],
        fields: FIELDS,
    };

    assert_eq!(meta.fields.len(), 5);

    // Verify all fields are accessible
    for (i, field) in meta.fields.iter().enumerate() {
        assert_eq!(field.name, format!("field{}", i + 1));
    }
}

#[test]
fn test_event_meta_discriminator_formats() {
    // Test various discriminator patterns
    let discriminators = [
        [0, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [255, 255, 255, 255, 255, 255, 255, 255],
        [1, 2, 3, 4, 5, 6, 7, 8],
        [128, 0, 0, 0, 0, 0, 0, 0],
    ];

    for disc in discriminators {
        let meta = EventMeta {
            name: "TestEvent",
            discriminator: disc,
            fields: &[],
        };
        assert_eq!(meta.discriminator, disc);
    }
}

#[test]
fn test_event_meta_is_copy() {
    static FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "value",
        type_name: "u64",
    }];

    let meta1 = EventMeta {
        name: "CopyEvent",
        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
        fields: FIELDS,
    };

    let meta2 = meta1; // Should copy
    assert_eq!(meta1.name, meta2.name);
    assert_eq!(meta1.discriminator, meta2.discriminator);
}

#[test]
fn test_event_meta_debug_format() {
    static FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "test",
        type_name: "u64",
    }];

    let meta = EventMeta {
        name: "DebugEvent",
        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
        fields: FIELDS,
    };

    // Verify Debug trait is implemented
    let debug_str = format!("{:?}", meta);
    assert!(debug_str.contains("EventMeta"));
}

#[test]
fn test_event_field_meta_debug_format() {
    let meta = EventFieldMeta {
        name: "test_field",
        type_name: "u64",
    };

    let debug_str = format!("{:?}", meta);
    assert!(debug_str.contains("EventFieldMeta"));
}

#[test]
fn test_event_meta_with_padding_field() {
    static FIELDS: &[EventFieldMeta] = &[
        EventFieldMeta {
            name: "value",
            type_name: "u64",
        },
        EventFieldMeta {
            name: "flag",
            type_name: "u8",
        },
        EventFieldMeta {
            name: "_padding",
            type_name: "[u8; 7]",
        },
    ];

    let meta = EventMeta {
        name: "PaddedEvent",
        discriminator: [77, 77, 77, 77, 77, 77, 77, 77],
        fields: FIELDS,
    };

    // Padding field should be present in metadata
    assert_eq!(meta.fields[2].name, "_padding");
    assert_eq!(meta.fields[2].type_name, "[u8; 7]");
}

#[test]
fn test_event_meta_with_nested_arrays() {
    static FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "matrix",
        type_name: "[[u64; 4]; 4]",
    }];

    let meta = EventMeta {
        name: "MatrixEvent",
        discriminator: [66, 66, 66, 66, 66, 66, 66, 66],
        fields: FIELDS,
    };

    assert_eq!(meta.fields[0].type_name, "[[u64; 4]; 4]");
}

#[test]
fn test_multiple_event_metas() {
    static FIELDS1: &[EventFieldMeta] = &[EventFieldMeta {
        name: "value1",
        type_name: "u64",
    }];

    static FIELDS2: &[EventFieldMeta] = &[EventFieldMeta {
        name: "value2",
        type_name: "u128",
    }];

    let meta1 = EventMeta {
        name: "Event1",
        discriminator: [1, 1, 1, 1, 1, 1, 1, 1],
        fields: FIELDS1,
    };

    let meta2 = EventMeta {
        name: "Event2",
        discriminator: [2, 2, 2, 2, 2, 2, 2, 2],
        fields: FIELDS2,
    };

    assert_ne!(meta1.name, meta2.name);
    assert_ne!(meta1.discriminator, meta2.discriminator);
}
