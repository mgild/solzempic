//! Tests for Event trait implementation

use crate::event::Event;
use bytemuck::{Pod, Zeroable};
use solana_address::Address;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SimpleEvent {
    pub value: u64,
}

impl Event for SimpleEvent {
    const DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    const NAME: &'static str = "SimpleEvent";
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ComplexEvent {
    pub address: Address,
    pub amount: u64,
    pub flag: u8,
    pub _padding: [u8; 7],
}

impl Event for ComplexEvent {
    const DISCRIMINATOR: [u8; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    const NAME: &'static str = "ComplexEvent";
}

#[test]
fn test_event_trait_constants() {
    assert_eq!(SimpleEvent::DISCRIMINATOR, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(SimpleEvent::NAME, "SimpleEvent");
    assert_eq!(ComplexEvent::DISCRIMINATOR, [10, 20, 30, 40, 50, 60, 70, 80]);
    assert_eq!(ComplexEvent::NAME, "ComplexEvent");
}

#[test]
fn test_event_name_uniqueness() {
    assert_ne!(SimpleEvent::NAME, ComplexEvent::NAME);
}

#[test]
fn test_event_discriminator_uniqueness() {
    assert_ne!(SimpleEvent::DISCRIMINATOR, ComplexEvent::DISCRIMINATOR);
}

#[test]
fn test_event_discriminator_length() {
    assert_eq!(SimpleEvent::DISCRIMINATOR.len(), 8);
    assert_eq!(ComplexEvent::DISCRIMINATOR.len(), 8);
}

#[test]
fn test_event_is_copy() {
    let event1 = SimpleEvent { value: 42 };
    let event2 = event1; // Should copy, not move
    assert_eq!(event1.value, event2.value);
}

#[test]
fn test_event_is_clone() {
    let event1 = ComplexEvent {
        address: Address::new_from_array([1u8; 32]),
        amount: 1000,
        flag: 1,
        _padding: [0; 7],
    };
    let event2 = event1.clone();
    assert_eq!(event1.amount, event2.amount);
    assert_eq!(event1.flag, event2.flag);
}

#[test]
fn test_multiple_event_types() {
    // Verify we can have multiple event types in the same scope
    let _simple = SimpleEvent { value: 100 };
    let _complex = ComplexEvent {
        address: Address::new_from_array([2u8; 32]),
        amount: 200,
        flag: 1,
        _padding: [0; 7],
    };
    // Both should coexist without conflicts
}

#[test]
fn test_event_with_max_discriminator_values() {
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct MaxDiscEvent {
        pub value: u8,
    }

    impl Event for MaxDiscEvent {
        const DISCRIMINATOR: [u8; 8] = [255, 255, 255, 255, 255, 255, 255, 255];
        const NAME: &'static str = "MaxDiscEvent";
    }

    assert_eq!(
        MaxDiscEvent::DISCRIMINATOR,
        [255, 255, 255, 255, 255, 255, 255, 255]
    );
}

#[test]
fn test_event_with_zero_discriminator() {
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct ZeroDiscEvent {
        pub value: u8,
    }

    impl Event for ZeroDiscEvent {
        const DISCRIMINATOR: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
        const NAME: &'static str = "ZeroDiscEvent";
    }

    assert_eq!(ZeroDiscEvent::DISCRIMINATOR, [0, 0, 0, 0, 0, 0, 0, 0]);
}
