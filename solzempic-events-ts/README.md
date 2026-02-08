# @solzempic/events

TypeScript SDK for parsing Solzempic events from Solana transaction logs.

## Features

- 🚀 **Zero-overhead**: Parses C struct layout events with no serialization overhead
- 🔒 **Type-safe**: Full TypeScript support with IDL-based type generation
- 📡 **Real-time**: Subscribe to events as they happen
- 🎯 **Anchor-compatible**: Event discriminators match Anchor's format
- ⚡ **Fast**: Efficient discriminator-based lookup and decoding

## Installation

```bash
npm install @solzempic/events
# or
yarn add @solzempic/events
# or
pnpm add @solzempic/events
```

## Usage

### Parse Events from Transaction

```typescript
import { createEventParser } from '@solzempic/events';
import { Connection } from '@solana/web3.js';
import idl from './your-program-idl.json';

// Create parser from IDL
const parser = createEventParser(idl);

// Parse events from transaction signature
const connection = new Connection('https://api.mainnet-beta.solana.com');
const signature = 'your-transaction-signature';

const events = await parser.parseTransaction(connection, signature);

for (const event of events) {
  console.log(`Event: ${event.name}`);
  console.log('Data:', event.data);
  console.log('Slot:', event.slot);
  console.log('Block Time:', event.blockTime);
}
```

### Subscribe to Real-Time Events

```typescript
import { createEventParser } from '@solzempic/events';
import { Connection, PublicKey } from '@solana/web3.js';
import idl from './your-program-idl.json';

const parser = createEventParser(idl);
const connection = new Connection('https://api.mainnet-beta.solana.com');
const programId = new PublicKey('Your1111111111111111111111111111111111111');

// Subscribe to all events
const subscriptionId = await parser.subscribe(
  connection,
  programId,
  (event) => {
    console.log('New event:', event.name, event.data);
  }
);

// Unsubscribe later
await parser.unsubscribe(connection, subscriptionId);
```

### Filter Events by Name

```typescript
// Only parse specific event types
const events = await parser.parseTransaction(connection, signature, {
  eventNames: ['TransferEvent', 'SwapEvent'],
});
```

### Parse Logs Directly

```typescript
// If you already have transaction logs
const logs = [
  'Program data: <base64-encoded-event>',
  // ... more logs
];

const events = parser.parseLogs(logs);
```

## Event Format

Solzempic events use C struct layout for zero-overhead serialization:

```rust
// Rust event definition
#[event]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub timestamp: i64,
}
```

The event is logged with an 8-byte discriminator (SHA256 hash) followed by the struct data in little-endian format.

## Supported Types

### Primitive Types
- `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`
- `f32`, `f64`
- `bool`
- `pubkey` (Solana public key)

### Composite Types
- Fixed-size arrays: `[type; N]`
- Nested structs (defined types)

**Note**: Variable-length types (`String`, `Vec<T>`) are not supported in C struct events due to their zero-overhead design.

## Performance

Event parsing is highly efficient:
- **Discriminator lookup**: O(1) hash map lookup
- **Decoding**: Zero-copy reads from buffer
- **Memory**: No allocations for primitive types

## License

MIT

## Repository

https://github.com/mgild/solzempic
