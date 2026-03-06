//! Event emission and parsing for Solana programs.
//!
//! This module provides zero-overhead event logging with C struct serialization.
//! Events are base64-encoded and logged via `sol_log_data`, making them easy
//! to parse from transaction logs.
//!
//! # Event Format
//!
//! Events use the following binary layout:
//! - 8-byte discriminator (SHA256 hash of "event:<EventName>")
//! - Event struct data (C layout via `#[repr(C)]`)
//!
//! # Performance
//!
//! Events are zero-overhead:
//! - No serialization overhead (direct memory cast via bytemuck)
//! - Emit cost: ~1000 CUs + size overhead
//! - Events are `Pod` types - no allocations, no copies
//!
//! # Usage
//!
//! ```ignore
//! use solzempic::event;
//!
//! #[event]
//! pub struct TransferEvent {
//!     pub from: Address,
//!     pub to: Address,
//!     pub amount: u64,
//! }
//!
//! // In your instruction execute():
//! emit!(TransferEvent {
//!     from: *source.key(),
//!     to: *destination.key(),
//!     amount: params.amount,
//! });
//! ```

use bytemuck::Pod;

/// A single data slice for the sol_log_data syscall.
/// The syscall expects an array of these, NOT raw bytes.
#[repr(C)]
struct SolBytes {
    addr: *const u8,
    len: u64,
}

// Solana syscall for logging data (not directly exported by pinocchio)
extern "C" {
    fn sol_log_data(data: *const SolBytes, data_len: u64);
}

/// Trait for types that can be emitted as events.
///
/// This trait is automatically implemented by the `#[event]` macro.
/// It provides the discriminator for event types.
///
/// Events must be `Pod` types (Plain Old Data) for zero-copy serialization.
pub trait Event: Pod {
    /// 8-byte event discriminator (SHA256 of "event:<EventName>")
    const DISCRIMINATOR: [u8; 8];

    /// Event name for IDL generation
    const NAME: &'static str;
}

/// Emit an event to the transaction log.
///
/// This function logs the event with its discriminator via `sol_log_data`
/// syscall. The event is zero-copy serialized using bytemuck.
///
/// # Example
///
/// ```ignore
/// emit_event(&TransferEvent {
///     from: *source.key(),
///     to: *destination.key(),
///     amount: 1000,
/// });
/// ```
///
/// # Compute Cost
///
/// - Serialization: 0 CUs (zero-copy via bytemuck)
/// - Logging: ~1000 CUs + size overhead
///
/// Events are relatively expensive, so use them sparingly for important
/// state transitions that off-chain systems need to observe.
#[inline]
pub fn emit_event<T: Event>(event: &T) {
    // Zero-copy: pass discriminator and event data as two separate slices
    // to sol_log_data, avoiding any memcpy or buffer allocation.
    let disc = T::DISCRIMINATOR;
    let event_bytes = bytemuck::bytes_of(event);

    let sol_bytes = [
        SolBytes {
            addr: disc.as_ptr(),
            len: 8,
        },
        SolBytes {
            addr: event_bytes.as_ptr(),
            len: event_bytes.len() as u64,
        },
    ];
    unsafe {
        sol_log_data(sol_bytes.as_ptr(), 2);
    }
}

/// Macro for emitting events with ergonomic syntax.
///
/// This macro wraps `emit_event` to provide a cleaner API.
///
/// # Example
///
/// ```ignore
/// emit!(TransferEvent {
///     from: *source.key(),
///     to: *destination.key(),
///     amount: params.amount,
/// });
/// ```
#[macro_export]
macro_rules! emit {
    ($event:expr) => {
        $crate::event::emit_event(&$event)
    };
}

/// Metadata for a single field in an event type definition.
#[derive(Clone, Copy, Debug)]
pub struct EventFieldMeta {
    /// Field name
    pub name: &'static str,
    /// Type name (e.g., "u64", "pubkey", "[u8; 32]")
    pub type_name: &'static str,
}

/// Metadata for an event type definition.
///
/// Used for generating the "events" section of the IDL.
#[derive(Clone, Copy, Debug)]
pub struct EventMeta {
    /// Event type name (e.g., "TransferEvent", "SwapEvent")
    pub name: &'static str,
    /// Discriminator (8-byte array with ID in first byte)
    pub discriminator: [u8; 8],
    /// Field metadata slice
    pub fields: &'static [EventFieldMeta],
}

/// Trait for event types that provide IDL metadata.
///
/// Implemented by `#[event]` macro when `idl` feature is enabled.
pub trait EventIdlMeta {
    /// Event type name
    const NAME: &'static str;
    /// 8-byte discriminator
    const DISCRIMINATOR: [u8; 8];
    /// Field metadata
    const FIELDS: &'static [EventFieldMeta];
    /// Complete event type metadata
    const META: EventMeta;
}

// Inventory collection for events (requires idl feature)
#[cfg(feature = "idl")]
inventory::collect!(&'static EventMeta);

#[cfg(test)]
mod tests {
    mod emit_tests;
    mod metadata_tests;
    mod pod_tests;
    mod trait_tests;
}
