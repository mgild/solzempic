/**
 * TypeScript types for Solzempic events and IDL
 */

import { PublicKey } from '@solana/web3.js';

/**
 * IDL field type definition
 */
export type IdlType =
  | 'bool'
  | 'u8'
  | 'i8'
  | 'u16'
  | 'i16'
  | 'u32'
  | 'i32'
  | 'f32'
  | 'u64'
  | 'i64'
  | 'f64'
  | 'u128'
  | 'i128'
  | 'pubkey'
  | 'string'
  | { array: [IdlType, number] }
  | { defined: string };

/**
 * Event field definition in IDL
 */
export interface IdlEventField {
  name: string;
  type: IdlType;
}

/**
 * Event definition in IDL
 */
export interface IdlEvent {
  name: string;
  discriminator: number[];
  fields: IdlEventField[];
}

/**
 * Solzempic program IDL
 */
export interface SolzempicIdl {
  address: string;
  metadata: {
    name: string;
    version: string;
    spec: string;
  };
  instructions: any[];
  accounts: any[];
  types: any[];
  events: IdlEvent[];
  errors: any[];
}

/**
 * Parsed event from transaction log
 */
export interface ParsedEvent<T = any> {
  name: string;
  data: T;
}

/**
 * Event with additional metadata
 */
export interface EventWithMetadata<T = any> extends ParsedEvent<T> {
  signature: string;
  slot: number;
  blockTime: number | null;
}

/**
 * Event parser options
 */
export interface ParserOptions {
  /**
   * Filter events by name
   */
  eventNames?: string[];
}

/**
 * Type mapping from IDL types to TypeScript types
 */
export type TypeMap = {
  bool: boolean;
  u8: number;
  i8: number;
  u16: number;
  i16: number;
  u32: number;
  i32: number;
  f32: number;
  u64: bigint;
  i64: bigint;
  f64: number;
  u128: bigint;
  i128: bigint;
  pubkey: PublicKey;
  string: string;
};
