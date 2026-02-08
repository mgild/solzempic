//! Tests for JSON IDL generation

use crate::idl::{to_json, to_json_full_with_events};
use crate::{EventFieldMeta, EventMeta};
use alloc::vec;

#[test]
fn test_basic_idl_generation() {
    let json = to_json("Test111111111111111111111111111111111111111", "test", "0.1.0", &[]);

    assert!(json.contains("\"address\": \"Test111111111111111111111111111111111111111\""));
    assert!(json.contains("\"name\": \"test\""));
    assert!(json.contains("\"version\": \"0.1.0\""));
    assert!(json.contains("\"instructions\": []"));
}

#[test]
fn test_idl_with_events() {
    static EVENT_FIELDS: &[EventFieldMeta] = &[
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

    static EVENT_META: EventMeta = EventMeta {
        name: "TransferEvent",
        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
        fields: EVENT_FIELDS,
    };

    let events = vec![&EVENT_META];

    let json = to_json_full_with_events(
        "Test111111111111111111111111111111111111111",
        "test",
        "0.1.0",
        &[],
        &[],
        &events,
    );

    assert!(json.contains("\"events\": ["));
    assert!(json.contains("\"name\": \"TransferEvent\""));
    assert!(json.contains("\"discriminator\": [1, 2, 3, 4, 5, 6, 7, 8]"));
    assert!(json.contains("\"from\""));
    assert!(json.contains("\"to\""));
    assert!(json.contains("\"amount\""));
}

#[test]
fn test_idl_with_multiple_events() {
    static EVENT1_FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "value",
        type_name: "u64",
    }];

    static EVENT2_FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "flag",
        type_name: "bool",
    }];

    static EVENT1: EventMeta = EventMeta {
        name: "Event1",
        discriminator: [1; 8],
        fields: EVENT1_FIELDS,
    };

    static EVENT2: EventMeta = EventMeta {
        name: "Event2",
        discriminator: [2; 8],
        fields: EVENT2_FIELDS,
    };

    let events = vec![&EVENT1, &EVENT2];

    let json = to_json_full_with_events(
        "Test111111111111111111111111111111111111111",
        "test",
        "0.1.0",
        &[],
        &[],
        &events,
    );

    assert!(json.contains("\"name\": \"Event1\""));
    assert!(json.contains("\"name\": \"Event2\""));
    assert!(json.contains("\"discriminator\": [1, 1, 1, 1, 1, 1, 1, 1]"));
    assert!(json.contains("\"discriminator\": [2, 2, 2, 2, 2, 2, 2, 2]"));
}

#[test]
fn test_idl_events_section_format() {
    static EVENT_FIELDS: &[EventFieldMeta] = &[EventFieldMeta {
        name: "timestamp",
        type_name: "i64",
    }];

    static EVENT: EventMeta = EventMeta {
        name: "TimestampEvent",
        discriminator: [99; 8],
        fields: EVENT_FIELDS,
    };

    let events = vec![&EVENT];

    let json = to_json_full_with_events(
        "Test111111111111111111111111111111111111111",
        "test",
        "0.1.0",
        &[],
        &[],
        &events,
    );

    // Verify JSON structure
    assert!(json.contains("\"events\": ["));
    assert!(json.contains("{"));
    assert!(json.contains("\"name\": \"TimestampEvent\""));
    assert!(json.contains("\"fields\": ["));
    assert!(json.contains("}"));
}

#[test]
fn test_idl_with_no_events() {
    let json = to_json_full_with_events(
        "Test111111111111111111111111111111111111111",
        "test",
        "0.1.0",
        &[],
        &[],
        &[],
    );

    assert!(json.contains("\"events\": []"));
}

#[test]
fn test_idl_event_field_types() {
    static FIELDS: &[EventFieldMeta] = &[
        EventFieldMeta {
            name: "u8_field",
            type_name: "u8",
        },
        EventFieldMeta {
            name: "u64_field",
            type_name: "u64",
        },
        EventFieldMeta {
            name: "address_field",
            type_name: "Address",
        },
        EventFieldMeta {
            name: "array_field",
            type_name: "[u8; 32]",
        },
    ];

    static EVENT: EventMeta = EventMeta {
        name: "TypesEvent",
        discriminator: [77; 8],
        fields: FIELDS,
    };

    let events = vec![&EVENT];

    let json = to_json_full_with_events(
        "Test111111111111111111111111111111111111111",
        "test",
        "0.1.0",
        &[],
        &[],
        &events,
    );

    assert!(json.contains("\"u8Field\""));
    assert!(json.contains("\"u64Field\""));
    assert!(json.contains("\"addressField\""));
    assert!(json.contains("\"arrayField\""));
}

#[test]
fn test_idl_discriminator_format() {
    static FIELDS: &[EventFieldMeta] = &[];

    static EVENT: EventMeta = EventMeta {
        name: "TestEvent",
        discriminator: [10, 20, 30, 40, 50, 60, 70, 80],
        fields: FIELDS,
    };

    let events = vec![&EVENT];

    let json = to_json_full_with_events(
        "Test111111111111111111111111111111111111111",
        "test",
        "0.1.0",
        &[],
        &[],
        &events,
    );

    // Verify discriminator is formatted as array
    assert!(json.contains("\"discriminator\": [10, 20, 30, 40, 50, 60, 70, 80]"));
}

#[test]
fn test_idl_camel_case_conversion() {
    static FIELDS: &[EventFieldMeta] = &[
        EventFieldMeta {
            name: "snake_case_field",
            type_name: "u64",
        },
        EventFieldMeta {
            name: "another_field_name",
            type_name: "u32",
        },
    ];

    static EVENT: EventMeta = EventMeta {
        name: "CamelCaseEvent",
        discriminator: [55; 8],
        fields: FIELDS,
    };

    let events = vec![&EVENT];

    let json = to_json_full_with_events(
        "Test111111111111111111111111111111111111111",
        "test",
        "0.1.0",
        &[],
        &[],
        &events,
    );

    // Verify snake_case is converted to camelCase in JSON
    assert!(json.contains("\"snakeCaseField\""));
    assert!(json.contains("\"anotherFieldName\""));
}

#[test]
fn test_idl_complete_structure() {
    let json = to_json("Test111111111111111111111111111111111111111", "test", "0.1.0", &[]);

    // Verify all required sections
    assert!(json.contains("\"address\":"));
    assert!(json.contains("\"metadata\":"));
    assert!(json.contains("\"instructions\":"));
    assert!(json.contains("\"accounts\":"));
    assert!(json.contains("\"types\":"));
    assert!(json.contains("\"events\":"));
    assert!(json.contains("\"errors\":"));
}

#[test]
fn test_idl_valid_json_structure() {
    let json = to_json("Test111111111111111111111111111111111111111", "test", "0.1.0", &[]);

    // Verify JSON starts and ends correctly
    assert!(json.starts_with('{'));
    assert!(json.ends_with("}\n"));

    // Verify it contains expected structure
    assert!(json.contains("\"spec\": \"0.1.0\""));
}
