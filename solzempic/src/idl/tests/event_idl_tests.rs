//! Tests for event IDL generation

use crate::{EventFieldMeta, EventMeta};
use alloc::vec;

#[test]
fn test_event_meta_structure() {
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
    assert_eq!(meta.discriminator.len(), 8);
    assert_eq!(meta.fields.len(), 3);
}

#[test]
fn test_event_field_meta_primitives() {
    let field = EventFieldMeta {
        name: "value",
        type_name: "u64",
    };

    assert_eq!(field.name, "value");
    assert_eq!(field.type_name, "u64");
}

#[test]
fn test_event_field_meta_arrays() {
    let field = EventFieldMeta {
        name: "bytes",
        type_name: "[u8; 32]",
    };

    assert_eq!(field.name, "bytes");
    assert_eq!(field.type_name, "[u8; 32]");
}

#[test]
fn test_event_meta_with_no_fields() {
    static FIELDS: &[EventFieldMeta] = &[];

    let meta = EventMeta {
        name: "EmptyEvent",
        discriminator: [0; 8],
        fields: FIELDS,
    };

    assert_eq!(meta.fields.len(), 0);
}

#[test]
fn test_event_meta_with_single_field() {
    static FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "timestamp",
        type_name: "i64",
    }];

    let meta = EventMeta {
        name: "TimestampEvent",
        discriminator: [99; 8],
        fields: FIELDS,
    };

    assert_eq!(meta.fields.len(), 1);
    assert_eq!(meta.fields[0].name, "timestamp");
    assert_eq!(meta.fields[0].type_name, "i64");
}

#[test]
fn test_event_meta_with_many_fields() {
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
            type_name: "u128",
        },
        EventFieldMeta {
            name: "field6",
            type_name: "Address",
        },
        EventFieldMeta {
            name: "field7",
            type_name: "[u8; 32]",
        },
    ];

    let meta = EventMeta {
        name: "ComplexEvent",
        discriminator: [88; 8],
        fields: FIELDS,
    };

    assert_eq!(meta.fields.len(), 7);
}

#[test]
fn test_event_discriminator_uniqueness() {
    static FIELDS1: &[EventFieldMeta] = &[];
    static FIELDS2: &[EventFieldMeta] = &[];

    let meta1 = EventMeta {
        name: "Event1",
        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
        fields: FIELDS1,
    };

    let meta2 = EventMeta {
        name: "Event2",
        discriminator: [8, 7, 6, 5, 4, 3, 2, 1],
        fields: FIELDS2,
    };

    assert_ne!(meta1.discriminator, meta2.discriminator);
}

#[test]
fn test_event_name_formats() {
    let names = vec![
        "SimpleEvent",
        "ComplexEventName",
        "EVENT_WITH_UNDERSCORES",
        "eventCamelCase",
        "Event123",
    ];

    for name in names {
        static FIELDS: &[EventFieldMeta] = &[];
        let meta = EventMeta {
            name,
            discriminator: [0; 8],
            fields: FIELDS,
        };
        assert_eq!(meta.name, name);
    }
}

#[test]
fn test_event_field_type_coverage() {
    let types = vec![
        "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f32", "f64",
        "bool", "Address", "Pubkey", "[u8; 32]", "[u64; 4]", "CustomType",
    ];

    for type_name in types {
        let field = EventFieldMeta {
            name: "test",
            type_name,
        };
        assert_eq!(field.type_name, type_name);
    }
}

#[test]
fn test_event_field_name_formats() {
    let names = vec![
        "simple",
        "camelCase",
        "snake_case",
        "PascalCase",
        "_private",
        "__dunder",
        "field123",
    ];

    for name in names {
        let field = EventFieldMeta {
            name,
            type_name: "u64",
        };
        assert_eq!(field.name, name);
    }
}

#[test]
fn test_multiple_events_collection() {
    static FIELDS1: &[EventFieldMeta] = &[EventFieldMeta {
        name: "value1",
        type_name: "u64",
    }];

    static FIELDS2: &[EventFieldMeta] = &[EventFieldMeta {
        name: "value2",
        type_name: "u128",
    }];

    static FIELDS3: &[EventFieldMeta] = &[EventFieldMeta {
        name: "value3",
        type_name: "Address",
    }];

    let events = vec![
        EventMeta {
            name: "Event1",
            discriminator: [1; 8],
            fields: FIELDS1,
        },
        EventMeta {
            name: "Event2",
            discriminator: [2; 8],
            fields: FIELDS2,
        },
        EventMeta {
            name: "Event3",
            discriminator: [3; 8],
            fields: FIELDS3,
        },
    ];

    assert_eq!(events.len(), 3);
    assert_ne!(events[0].discriminator, events[1].discriminator);
    assert_ne!(events[1].discriminator, events[2].discriminator);
}
