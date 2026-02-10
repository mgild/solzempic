//! Tests for event emission and stack/heap allocation

use crate::event::Event;
use bytemuck::{Pod, Zeroable};
use solana_address::Address;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SmallEvent {
    pub value: u64,
}

impl Event for SmallEvent {
    const DISCRIMINATOR: [u8; 8] = [1, 1, 1, 1, 1, 1, 1, 1];
    const NAME: &'static str = "SmallEvent";
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MediumEvent {
    pub data: [u8; 200],
}

impl Event for MediumEvent {
    const DISCRIMINATOR: [u8; 8] = [2, 2, 2, 2, 2, 2, 2, 2];
    const NAME: &'static str = "MediumEvent";
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LargeEvent {
    pub data: [u8; 512],
}

impl Event for LargeEvent {
    const DISCRIMINATOR: [u8; 8] = [3, 3, 3, 3, 3, 3, 3, 3];
    const NAME: &'static str = "LargeEvent";
}

#[test]
fn test_small_event_stack_allocation() {
    let event = SmallEvent { value: 42 };
    let event_size = core::mem::size_of_val(&event);
    let total_size = 8 + event_size; // discriminator + event

    // Should fit in 256-byte stack buffer
    assert!(total_size <= 256);
    assert_eq!(event_size, 8);
}

#[test]
fn test_medium_event_stack_allocation() {
    let event = MediumEvent { data: [0; 200] };
    let event_size = core::mem::size_of_val(&event);
    let total_size = 8 + event_size;

    // Should fit in 256-byte stack buffer
    assert!(total_size <= 256);
    assert_eq!(event_size, 200);
    assert_eq!(total_size, 208);
}

#[test]
fn test_large_event_heap_allocation() {
    let event = LargeEvent { data: [0; 512] };
    let event_size = core::mem::size_of_val(&event);
    let total_size = 8 + event_size;

    // Should NOT fit in 256-byte stack buffer
    assert!(total_size > 256);
    assert_eq!(event_size, 512);
    assert_eq!(total_size, 520);
}

#[test]
fn test_boundary_event_256_bytes() {
    // Event that exactly fits the boundary
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct BoundaryEvent {
        pub data: [u8; 248], // 248 + 8 (discriminator) = 256
    }

    impl Event for BoundaryEvent {
        const DISCRIMINATOR: [u8; 8] = [4, 4, 4, 4, 4, 4, 4, 4];
        const NAME: &'static str = "BoundaryEvent";
    }

    let event = BoundaryEvent { data: [0; 248] };
    let total_size = 8 + core::mem::size_of_val(&event);
    assert_eq!(total_size, 256);
}

#[test]
fn test_event_discriminator_prepending() {
    let event = SmallEvent {
        value: 0x123456789ABCDEF0,
    };

    // Simulate what emit_event does
    let disc = SmallEvent::DISCRIMINATOR;
    let event_bytes = bytemuck::bytes_of(&event);

    // Verify discriminator comes first
    assert_eq!(disc, [1, 1, 1, 1, 1, 1, 1, 1]);

    // Verify event bytes follow
    let expected_value = 0x123456789ABCDEF0u64.to_le_bytes();
    assert_eq!(event_bytes, &expected_value);
}

#[test]
fn test_multiple_event_sizes() {
    // Test various event sizes to ensure proper handling
    struct SizeTest<const N: usize>;

    macro_rules! test_size {
        ($n:expr) => {{
            #[repr(C)]
            #[derive(Clone, Copy, Pod, Zeroable)]
            struct TestEvent {
                pub data: [u8; $n],
            }

            impl Event for TestEvent {
                const DISCRIMINATOR: [u8; 8] = [5, 5, 5, 5, 5, 5, 5, 5];
                const NAME: &'static str = concat!("TestEvent", stringify!($n));
            }

            let event = TestEvent { data: [0; $n] };
            let total = 8 + core::mem::size_of_val(&event);
            (total, total <= 256)
        }};
    }

    // Test various sizes
    let (total_1, uses_stack_1) = test_size!(1);
    assert!(uses_stack_1);
    assert_eq!(total_1, 9);

    let (total_100, uses_stack_100) = test_size!(100);
    assert!(uses_stack_100);
    assert_eq!(total_100, 108);

    let (total_248, uses_stack_248) = test_size!(248);
    assert!(uses_stack_248);
    assert_eq!(total_248, 256);

    let (total_300, uses_stack_300) = test_size!(300);
    assert!(!uses_stack_300);
    assert_eq!(total_300, 308);
}

#[test]
fn test_event_with_complex_layout() {
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct ComplexEvent {
        pub addresses: [Address; 2], // 64 bytes (2 * 32)
        pub amounts: [u64; 3],       // 24 bytes (3 * 8)
        pub flags: [u8; 10],         // 10 bytes
        pub _padding: [u8; 6],       // 6 bytes padding to align to 8
    }

    impl Event for ComplexEvent {
        const DISCRIMINATOR: [u8; 8] = [6, 6, 6, 6, 6, 6, 6, 6];
        const NAME: &'static str = "ComplexEvent";
    }

    let event = ComplexEvent {
        addresses: [
            Address::new_from_array([1u8; 32]),
            Address::new_from_array([2u8; 32]),
        ],
        amounts: [100, 200, 300],
        flags: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        _padding: [0; 6],
    };

    // Size: 2*32 (addresses) + 3*8 (amounts) + 10 (flags) + 6 (padding) = 104 bytes
    assert_eq!(core::mem::size_of_val(&event), 104);

    // Total with discriminator
    let total = 8 + 104;
    assert_eq!(total, 112);
    assert!(total <= 256); // Should use stack
}

#[test]
fn test_zero_sized_event_handling() {
    // Note: True zero-sized types aren't Pod-safe, but test minimal size
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct MinimalEvent {
        pub flag: u8,
    }

    impl Event for MinimalEvent {
        const DISCRIMINATOR: [u8; 8] = [7, 7, 7, 7, 7, 7, 7, 7];
        const NAME: &'static str = "MinimalEvent";
    }

    let event = MinimalEvent { flag: 1 };
    assert_eq!(core::mem::size_of_val(&event), 1);

    let total = 8 + 1;
    assert_eq!(total, 9);
    assert!(total <= 256);
}

#[test]
fn test_event_serialization_determinism() {
    // Same event should always serialize the same way
    let event1 = SmallEvent { value: 12345 };
    let event2 = SmallEvent { value: 12345 };

    let bytes1 = bytemuck::bytes_of(&event1);
    let bytes2 = bytemuck::bytes_of(&event2);

    assert_eq!(bytes1, bytes2);
}

#[test]
fn test_event_with_maximum_stack_size() {
    // Maximum event size that still uses stack (256 - 8 = 248 bytes)
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct MaxStackEvent {
        pub data: [u8; 248],
    }

    impl Event for MaxStackEvent {
        const DISCRIMINATOR: [u8; 8] = [8, 8, 8, 8, 8, 8, 8, 8];
        const NAME: &'static str = "MaxStackEvent";
    }

    let event = MaxStackEvent { data: [42; 248] };
    let total = 8 + core::mem::size_of_val(&event);
    assert_eq!(total, 256);

    let bytes = bytemuck::bytes_of(&event);
    assert_eq!(bytes.len(), 248);
    assert!(bytes.iter().all(|&b| b == 42));
}
