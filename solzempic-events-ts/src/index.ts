/**
 * @solzempic/events - TypeScript SDK for parsing Solzempic events
 *
 * Zero-overhead event parsing for Solana programs built with Solzempic.
 * Supports C struct layout events with type-safe decoding.
 *
 * @example
 * ```typescript
 * import { createEventParser } from '@solzempic/events';
 * import { Connection, PublicKey } from '@solana/web3.js';
 * import idl from './idl.json';
 *
 * // Create parser from IDL
 * const parser = createEventParser(idl);
 *
 * // Parse events from transaction
 * const connection = new Connection('https://api.mainnet-beta.solana.com');
 * const events = await parser.parseTransaction(connection, signature);
 *
 * // Subscribe to real-time events
 * const programId = new PublicKey('Your111...');
 * const subId = await parser.subscribe(connection, programId, (event) => {
 *   console.log('Event:', event.name, event.data);
 * });
 *
 * // Unsubscribe
 * await parser.unsubscribe(connection, subId);
 * ```
 *
 * @packageDocumentation
 */

export { EventParser, createEventParser } from './parser';
export { EventDecoder } from './decoder';
export type {
  IdlType,
  IdlEventField,
  IdlEvent,
  SolzempicIdl,
  ParsedEvent,
  EventWithMetadata,
  ParserOptions,
  TypeMap,
} from './types';
