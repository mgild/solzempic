//! Tests for Pod safety and layout verification

use crate::event::Event;
use bytemuck::{Pod, Zeroable};
use solana_address::Address;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TestEvent {
    pub value: u64,
    pub flag: u8,
    pub _padding: [u8; 7],
}

impl Event for TestEvent {
    const DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    const NAME: &'static str = "TestEvent";
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
}

impl Event for TransferEvent {
    const DISCRIMINATOR: [u8; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    const NAME: &'static str = "TransferEvent";
}

#[test]
fn test_event_is_pod() {
    let event = TestEvent {
        value: 12345,
        flag: 1,
        _padding: [0; 7],
    };

    // Verify we can cast to bytes
    let bytes = bytemuck::bytes_of(&event);
    assert_eq!(bytes.len(), core::mem::size_of::<TestEvent>());

    // Verify we can cast back
    let restored: &TestEvent = bytemuck::from_bytes(bytes);
    assert_eq!(restored.value, 12345);
    assert_eq!(restored.flag, 1);
}

#[test]
fn test_event_zero_initialization() {
    let event = TestEvent::zeroed();
    assert_eq!(event.value, 0);
    assert_eq!(event.flag, 0);
    assert_eq!(event._padding, [0; 7]);
}

#[test]
fn test_event_memory_layout() {
    let event = TestEvent {
        value: 0x123456789ABCDEF0,
        flag: 0xFF,
        _padding: [0; 7],
    };

    let bytes = bytemuck::bytes_of(&event);

    // Verify value is at offset 0 (little-endian)
    let value_bytes = 0x123456789ABCDEF0u64.to_le_bytes();
    assert_eq!(&bytes[0..8], &value_bytes);

    // Verify flag is at offset 8
    assert_eq!(bytes[8], 0xFF);

    // Verify padding is zeros
    assert_eq!(&bytes[9..16], &[0u8; 7]);
}

#[test]
fn test_transfer_event_layout() {
    let from = Address::new_from_array([1u8; 32]);
    let to = Address::new_from_array([2u8; 32]);

    let event = TransferEvent {
        from,
        to,
        amount: 1000000,
    };

    let bytes = bytemuck::bytes_of(&event);

    // Verify layout: 32 bytes (from) + 32 bytes (to) + 8 bytes (amount) = 72
    assert_eq!(bytes.len(), 72);

    // Verify from pubkey at offset 0
    assert_eq!(&bytes[0..32], from.as_ref());

    // Verify to pubkey at offset 32
    assert_eq!(&bytes[32..64], to.as_ref());

    // Verify amount at offset 64 (little-endian)
    let amount_bytes = 1000000u64.to_le_bytes();
    assert_eq!(&bytes[64..72], &amount_bytes);
}

#[test]
fn test_event_alignment() {
    // Verify proper alignment for different types
    assert_eq!(core::mem::align_of::<TestEvent>(), 8); // u64 alignment
    assert_eq!(core::mem::align_of::<TransferEvent>(), 1); // Address is [u8; 32]
}

#[test]
fn test_event_size_calculation() {
    // TestEvent: u64 (8) + u8 (1) + [u8; 7] (7) = 16 bytes
    assert_eq!(core::mem::size_of::<TestEvent>(), 16);

    // TransferEvent: Address (32) + Address (32) + u64 (8) = 72 bytes
    assert_eq!(core::mem::size_of::<TransferEvent>(), 72);
}

#[test]
fn test_event_is_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<TestEvent>();
    assert_sync::<TestEvent>();
    assert_send::<TransferEvent>();
    assert_sync::<TransferEvent>();
}

#[test]
fn test_event_slice_casting() {
    let events = [
        TestEvent {
            value: 1,
            flag: 1,
            _padding: [0; 7],
        },
        TestEvent {
            value: 2,
            flag: 2,
            _padding: [0; 7],
        },
        TestEvent {
            value: 3,
            flag: 3,
            _padding: [0; 7],
        },
    ];

    // Cast slice to bytes
    let bytes = bytemuck::cast_slice::<TestEvent, u8>(&events);
    assert_eq!(bytes.len(), 3 * core::mem::size_of::<TestEvent>());

    // Cast back to slice
    let restored = bytemuck::cast_slice::<u8, TestEvent>(bytes);
    assert_eq!(restored.len(), 3);
    assert_eq!(restored[0].value, 1);
    assert_eq!(restored[1].value, 2);
    assert_eq!(restored[2].value, 3);
}

#[test]
fn test_event_with_u64_types() {
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct U64Event {
        pub val1: u64,
        pub val2: u64,
        pub val3: u64,
    }

    impl Event for U64Event {
        const DISCRIMINATOR: [u8; 8] = [99, 99, 99, 99, 99, 99, 99, 99];
        const NAME: &'static str = "U64Event";
    }

    let event = U64Event {
        val1: 18446744073709551615,
        val2: 12345678900,
        val3: 987654321,
    };

    let bytes = bytemuck::bytes_of(&event);
    let restored: &U64Event = bytemuck::from_bytes(bytes);

    assert_eq!(restored.val1, 18446744073709551615);
    assert_eq!(restored.val2, 12345678900);
    assert_eq!(restored.val3, 987654321);
}

#[test]
fn test_event_with_i64_types() {
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct I64Event {
        pub val1: i64,
        pub val2: i64,
    }

    impl Event for I64Event {
        const DISCRIMINATOR: [u8; 8] = [98, 98, 98, 98, 98, 98, 98, 98];
        const NAME: &'static str = "I64Event";
    }

    let event = I64Event {
        val1: -9223372036854775808,
        val2: 9223372036854775807,
    };

    let bytes = bytemuck::bytes_of(&event);
    let restored: &I64Event = bytemuck::from_bytes(bytes);

    assert_eq!(restored.val1, -9223372036854775808);
    assert_eq!(restored.val2, 9223372036854775807);
}

#[test]
fn test_event_with_arrays() {
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct ArrayEvent {
        pub bytes: [u8; 32],
        pub values: [u64; 4],
    }

    impl Event for ArrayEvent {
        const DISCRIMINATOR: [u8; 8] = [88, 88, 88, 88, 88, 88, 88, 88];
        const NAME: &'static str = "ArrayEvent";
    }

    let event = ArrayEvent {
        bytes: [42; 32],
        values: [1, 2, 3, 4],
    };

    let bytes = bytemuck::bytes_of(&event);
    let restored: &ArrayEvent = bytemuck::from_bytes(bytes);

    assert_eq!(restored.bytes, [42; 32]);
    assert_eq!(restored.values, [1, 2, 3, 4]);
}
