# Solzempic Event System

Complete implementation of zero-overhead event emission and parsing for Solana programs.

## Overview

The event system provides:
- **Rust**: Zero-overhead event emission using C struct layout
- **TypeScript**: Full event parser with IDL-based type safety
- **Performance**: Stack allocation for small events, direct syscalls, no serialization overhead

---

## Rust Implementation

### Core Features

#### 1. Event Module (`solzempic/src/event.rs`)

**Event Trait**:
```rust
pub trait Event: Pod {
    const DISCRIMINATOR: [u8; 8];
    const NAME: &'static str;
}
```

**Emission**:
- `emit_event<T: Event>(event: &T)` - Emits event via `sol_log_data` syscall
- Stack allocation for events < 256 bytes (zero heap allocation)
- Zero-copy serialization via bytemuck

**Metadata**:
- `EventMeta` - IDL metadata for events
- `EventFieldMeta` - Field-level metadata
- `EventIdlMeta` - Trait for IDL generation
- Inventory-based auto-collection for IDL

#### 2. Event Macro (`solzempic-macros/src/lib.rs`)

**`#[event]` attribute macro**:
- Generates `#[repr(C)]` struct layout
- Implements `Pod` + `Zeroable` traits
- Calculates 8-byte SHA256-based discriminator
- Generates IDL metadata
- Auto-registers with inventory

**Discriminator generation**:
- SHA256("event:<EventName>") - Anchor-compatible format
- Stack allocation for name concatenation (< 128 chars)
- Const global for "event:" prefix
- First 8 bytes of hash used as discriminator

#### 3. IDL Generation (`solzempic/src/idl.rs`)

**Functions**:
- `to_json_with_accounts()` - Auto-collects events via inventory
- `to_json_full_with_events()` - Explicit event specification
- Events included in JSON IDL with discriminators and fields

**IDL Format**:
```json
{
  "events": [
    {
      "name": "TransferEvent",
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8],
      "fields": [
        { "name": "from", "type": "pubkey" },
        { "name": "to", "type": "pubkey" },
        { "name": "amount", "type": "u64" }
      ]
    }
  ]
}
```

### Usage Example

```rust
use solzempic::{event, emit, Event};
use solana_address::Address;

#[event]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub timestamp: i64,
}

// In your instruction execute():
emit!(TransferEvent {
    from: *source.key(),
    to: *destination.key(),
    amount: params.amount,
    timestamp: clock.unix_timestamp,
});
```

### Performance

- **Serialization**: 0 CUs (zero-copy via bytemuck)
- **Logging**: ~1000 CUs + size overhead
- **Memory**: Stack allocation for events < 256 bytes
- **Discriminator calc**: Build-time (zero runtime cost)

### Testing

**Location**: `solzempic/src/event/tests.rs`

**Coverage**:
- ✓ Event trait implementation
- ✓ Pod type verification
- ✓ C struct layout verification
- ✓ Multiple field decoding
- ✓ Event metadata
- ✓ Stack/heap allocation thresholds
- ✓ Discriminator uniqueness
- ✓ Zero initialization

**Run tests**:
```bash
cargo test --lib event --all-features
```

---

## TypeScript SDK

### Package: `@solzempic/events`

**Location**: `solzempic-events-ts/`

### Core Components

#### 1. EventParser (`src/parser.ts`)

**Features**:
- Parse events from transaction logs
- Parse events from transaction signatures
- Real-time event subscription
- Event filtering by name
- Discriminator-based fast lookup

**API**:
```typescript
const parser = createEventParser(idl);

// Parse from transaction
const events = await parser.parseTransaction(connection, signature);

// Subscribe to real-time events
const subId = await parser.subscribe(connection, programId, (event) => {
  console.log(event.name, event.data);
});

// Parse logs directly
const events = parser.parseLogs(logs, { eventNames: ['TransferEvent'] });
```

#### 2. EventDecoder (`src/decoder.ts`)

**Features**:
- Zero-copy C struct decoding
- Little-endian primitive types
- Fixed-size array support
- PublicKey handling
- Type-safe field decoding

**Supported Types**:
- Primitives: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`, `f32`, `f64`, `bool`
- Solana: `pubkey` (32-byte public keys)
- Arrays: `[type; N]` (fixed-size)

#### 3. Types (`src/types.ts`)

**IDL Types**:
- `SolzempicIdl` - Full program IDL
- `IdlEvent` - Event definition
- `IdlEventField` - Event field
- `ParsedEvent<T>` - Decoded event
- `EventWithMetadata<T>` - Event with transaction metadata

### Installation

```bash
npm install @solzempic/events
```

### Usage Examples

#### Parse Transaction Events

```typescript
import { createEventParser } from '@solzempic/events';
import { Connection } from '@solana/web3.js';
import idl from './idl.json';

const parser = createEventParser(idl);
const connection = new Connection('https://api.mainnet-beta.solana.com');

const events = await parser.parseTransaction(connection, signature);

for (const event of events) {
  console.log(`Event: ${event.name}`);
  console.log('Data:', event.data);
  console.log('Slot:', event.slot);
}
```

#### Real-Time Subscription

```typescript
import { createEventParser } from '@solzempic/events';
import { Connection, PublicKey } from '@solana/web3.js';

const parser = createEventParser(idl);
const connection = new Connection('https://api.mainnet-beta.solana.com');
const programId = new PublicKey('Your111...');

const subId = await parser.subscribe(
  connection,
  programId,
  (event) => {
    console.log('New event:', event.name, event.data);
  },
  { eventNames: ['TransferEvent'] } // Optional filter
);

// Unsubscribe later
await parser.unsubscribe(connection, subId);
```

### Testing

**Location**: `solzempic-events-ts/src/tests/`

**Coverage**:
- ✓ EventDecoder: All primitive types
- ✓ EventDecoder: Arrays and composite types
- ✓ EventDecoder: Error handling
- ✓ EventParser: Log parsing
- ✓ EventParser: Discriminator matching
- ✓ EventParser: Event filtering
- ✓ EventParser: Error resilience

**Run tests**:
```bash
cd solzempic-events-ts
npm install
npm test
```

---

## Event Format

### Binary Layout

```
+------------------------+
| Discriminator (8 bytes)|  SHA256("event:<Name>")
+------------------------+
| Event Data (N bytes)   |  C struct (#[repr(C)])
+------------------------+
```

### Discriminator Calculation

```rust
// Rust (build-time)
SHA256("event:TransferEvent") -> [u8; 8]

// TypeScript (parse-time)
Buffer.from([1, 2, 3, 4, 5, 6, 7, 8])
```

### Log Format

```
Program data: <base64-encoded-event>
```

---

## Dependencies

### Rust
- `bytemuck` - Zero-copy Pod types
- `sha2` - Discriminator hashing (build-time only)
- `inventory` - IDL auto-collection (optional, idl feature)
- `pinocchio` - Solana syscalls

### TypeScript
- `@solana/web3.js` - Solana connection
- `bs58` - Base58 encoding
- `buffer` - Buffer polyfill

---

## File Structure

```
solzempic/
├── src/
│   └── event.rs                 # Core event module
│       └── tests.rs             # Rust tests
├── idl.rs                        # IDL generation with events
└── ...

solzempic-macros/
└── src/
    └── lib.rs                    # #[event] macro

solzempic-events-ts/
├── src/
│   ├── types.ts                  # TypeScript types
│   ├── decoder.ts                # Event decoder
│   ├── parser.ts                 # Event parser
│   ├── index.ts                  # Exports
│   └── tests/
│       ├── decoder.test.ts       # Decoder tests
│       └── parser.test.ts        # Parser tests
├── package.json
├── tsconfig.json
├── jest.config.js
└── README.md
```

---

## Changelog Updates

Added to `CHANGELOG.md`:
- Complete event system implementation
- Zero-overhead C struct events
- IDL generation with event metadata
- TypeScript SDK with full parser
- Comprehensive test coverage

---

## Next Steps

### Optional Enhancements

1. **Codegen**: Generate TypeScript types from IDL events
2. **CLI Tool**: Event log viewer for debugging
3. **Benchmarks**: Performance testing vs Borsh serialization
4. **Examples**: Full example programs demonstrating events
5. **Filtering**: More advanced event filtering (by field values)

### Documentation

- [x] Rust docs (inline)
- [x] TypeScript docs (inline)
- [x] README for TypeScript package
- [x] This implementation document
- [ ] Video tutorial (optional)
- [ ] Blog post (optional)

---

## Summary

✅ **Rust**: Complete zero-overhead event framework
✅ **TypeScript**: Full-featured event parser SDK
✅ **Testing**: Comprehensive test coverage for both
✅ **Documentation**: Inline docs + README + this doc
✅ **Performance**: Optimized for minimal compute usage
✅ **Compatibility**: Anchor-compatible discriminators

The event system is production-ready and provides a complete solution for event emission and parsing in Solana programs built with Solzempic.
