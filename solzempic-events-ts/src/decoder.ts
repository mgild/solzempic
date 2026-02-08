/**
 * Event data decoder for Solzempic events
 *
 * Decodes C struct (POD) layout events from raw bytes
 */

import { PublicKey } from '@solana/web3.js';
import { IdlType, IdlEventField } from './types';

/**
 * Decoder for C struct layout events
 */
export class EventDecoder {
  private offset: number = 0;

  constructor(private data: Buffer) {}

  /**
   * Decode a complete event structure
   */
  decode(fields: IdlEventField[]): any {
    this.offset = 0;
    const result: any = {};

    for (const field of fields) {
      result[field.name] = this.decodeField(field.type);
    }

    return result;
  }

  /**
   * Decode a single field
   */
  private decodeField(type: IdlType): any {
    if (typeof type === 'string') {
      return this.decodePrimitive(type);
    } else if ('array' in type) {
      const [elemType, length] = type.array;
      if (elemType === 'u8') {
        // Special case: u8 array is just bytes
        const bytes = this.data.subarray(this.offset, this.offset + length);
        this.offset += length;
        return Array.from(bytes);
      } else {
        const arr = [];
        for (let i = 0; i < length; i++) {
          arr.push(this.decodeField(elemType));
        }
        return arr;
      }
    } else if ('defined' in type) {
      throw new Error(`Custom types not yet supported: ${type.defined}`);
    }

    throw new Error(`Unknown type: ${JSON.stringify(type)}`);
  }

  /**
   * Decode primitive types
   */
  private decodePrimitive(type: string): any {
    switch (type) {
      case 'bool':
        return this.readU8() !== 0;

      case 'u8':
        return this.readU8();

      case 'i8':
        return this.readI8();

      case 'u16':
        return this.readU16();

      case 'i16':
        return this.readI16();

      case 'u32':
        return this.readU32();

      case 'i32':
        return this.readI32();

      case 'f32':
        return this.readF32();

      case 'u64':
        return this.readU64();

      case 'i64':
        return this.readI64();

      case 'f64':
        return this.readF64();

      case 'u128':
        return this.readU128();

      case 'i128':
        return this.readI128();

      case 'pubkey':
        return this.readPubkey();

      case 'string':
        throw new Error('String type not supported in C struct events');

      default:
        throw new Error(`Unknown primitive type: ${type}`);
    }
  }

  // Read methods for each primitive type
  private readU8(): number {
    const value = this.data.readUInt8(this.offset);
    this.offset += 1;
    return value;
  }

  private readI8(): number {
    const value = this.data.readInt8(this.offset);
    this.offset += 1;
    return value;
  }

  private readU16(): number {
    const value = this.data.readUInt16LE(this.offset);
    this.offset += 2;
    return value;
  }

  private readI16(): number {
    const value = this.data.readInt16LE(this.offset);
    this.offset += 2;
    return value;
  }

  private readU32(): number {
    const value = this.data.readUInt32LE(this.offset);
    this.offset += 4;
    return value;
  }

  private readI32(): number {
    const value = this.data.readInt32LE(this.offset);
    this.offset += 4;
    return value;
  }

  private readF32(): number {
    const value = this.data.readFloatLE(this.offset);
    this.offset += 4;
    return value;
  }

  private readU64(): bigint {
    const value = this.data.readBigUInt64LE(this.offset);
    this.offset += 8;
    return value;
  }

  private readI64(): bigint {
    const value = this.data.readBigInt64LE(this.offset);
    this.offset += 8;
    return value;
  }

  private readF64(): number {
    const value = this.data.readDoubleLE(this.offset);
    this.offset += 8;
    return value;
  }

  private readU128(): bigint {
    // Read as two u64s (little-endian)
    const low = this.data.readBigUInt64LE(this.offset);
    const high = this.data.readBigUInt64LE(this.offset + 8);
    this.offset += 16;
    return (high << 64n) | low;
  }

  private readI128(): bigint {
    // Read as two u64s, handle sign
    const low = this.data.readBigUInt64LE(this.offset);
    const high = this.data.readBigInt64LE(this.offset + 8);
    this.offset += 16;
    return (high << 64n) | low;
  }

  private readPubkey(): PublicKey {
    const bytes = this.data.subarray(this.offset, this.offset + 32);
    this.offset += 32;
    return new PublicKey(bytes);
  }
}
