/**
 * Tests for EventDecoder
 */

import { EventDecoder } from '../src/decoder';
import { PublicKey } from '@solana/web3.js';
import { IdlEventField } from '../src/types';

describe('EventDecoder', () => {
  describe('Primitive types', () => {
    test('decodes u8', () => {
      const data = Buffer.from([42]);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'u8' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(42);
    });

    test('decodes i8', () => {
      const data = Buffer.from([0xff]); // -1
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'i8' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(-1);
    });

    test('decodes u16 (little-endian)', () => {
      const data = Buffer.from([0x34, 0x12]); // 0x1234 = 4660
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'u16' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(4660);
    });

    test('decodes u32 (little-endian)', () => {
      const data = Buffer.from([0x78, 0x56, 0x34, 0x12]); // 0x12345678
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'u32' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(0x12345678);
    });

    test('decodes u64 (little-endian)', () => {
      const data = Buffer.alloc(8);
      data.writeBigUInt64LE(BigInt('0x123456789ABCDEF0'));
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'u64' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(BigInt('0x123456789ABCDEF0'));
    });

    test('decodes i64 (little-endian)', () => {
      const data = Buffer.alloc(8);
      data.writeBigInt64LE(BigInt('-1234567890'));
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'i64' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(BigInt('-1234567890'));
    });

    test('decodes f32', () => {
      const data = Buffer.alloc(4);
      data.writeFloatLE(3.14159);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'f32' }];
      const result = decoder.decode(fields);
      expect(result.value).toBeCloseTo(3.14159, 5);
    });

    test('decodes f64', () => {
      const data = Buffer.alloc(8);
      data.writeDoubleLE(3.141592653589793);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'f64' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(3.141592653589793);
    });

    test('decodes bool (true)', () => {
      const data = Buffer.from([1]);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'flag', type: 'bool' }];
      const result = decoder.decode(fields);
      expect(result.flag).toBe(true);
    });

    test('decodes bool (false)', () => {
      const data = Buffer.from([0]);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'flag', type: 'bool' }];
      const result = decoder.decode(fields);
      expect(result.flag).toBe(false);
    });

    test('decodes pubkey', () => {
      const pubkeyBytes = Buffer.alloc(32, 1); // All 1s
      const decoder = new EventDecoder(pubkeyBytes);
      const fields: IdlEventField[] = [{ name: 'address', type: 'pubkey' }];
      const result = decoder.decode(fields);
      expect(result.address).toBeInstanceOf(PublicKey);
      expect(result.address.toBuffer()).toEqual(pubkeyBytes);
    });

    test('decodes u128', () => {
      const data = Buffer.alloc(16);
      // Write 0x12345678_9ABCDEF0_12345678_9ABCDEF0
      data.writeBigUInt64LE(BigInt('0x123456789ABCDEF0'), 0); // low
      data.writeBigUInt64LE(BigInt('0x123456789ABCDEF0'), 8); // high
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'value', type: 'u128' }];
      const result = decoder.decode(fields);
      expect(result.value).toBe(
        (BigInt('0x123456789ABCDEF0') << 64n) | BigInt('0x123456789ABCDEF0')
      );
    });
  });

  describe('Composite types', () => {
    test('decodes fixed-size u8 array', () => {
      const data = Buffer.from([1, 2, 3, 4, 5]);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'bytes', type: { array: ['u8', 5] } }];
      const result = decoder.decode(fields);
      expect(result.bytes).toEqual([1, 2, 3, 4, 5]);
    });

    test('decodes array of u32', () => {
      const data = Buffer.alloc(12); // 3 * 4 bytes
      data.writeUInt32LE(100, 0);
      data.writeUInt32LE(200, 4);
      data.writeUInt32LE(300, 8);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'values', type: { array: ['u32', 3] } }];
      const result = decoder.decode(fields);
      expect(result.values).toEqual([100, 200, 300]);
    });
  });

  describe('Multiple fields', () => {
    test('decodes struct with multiple fields', () => {
      // Struct: { value: u64, flag: u8, padding: [u8; 7] }
      const data = Buffer.alloc(16);
      data.writeBigUInt64LE(BigInt(12345), 0);
      data.writeUInt8(1, 8);
      // padding is zeros

      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [
        { name: 'value', type: 'u64' },
        { name: 'flag', type: 'u8' },
        { name: 'padding', type: { array: ['u8', 7] } },
      ];

      const result = decoder.decode(fields);
      expect(result.value).toBe(BigInt(12345));
      expect(result.flag).toBe(1);
      expect(result.padding).toEqual([0, 0, 0, 0, 0, 0, 0]);
    });

    test('decodes TransferEvent', () => {
      // TransferEvent: { from: pubkey, to: pubkey, amount: u64 }
      const data = Buffer.alloc(72); // 32 + 32 + 8
      const fromPubkey = Buffer.alloc(32, 1);
      const toPubkey = Buffer.alloc(32, 2);
      fromPubkey.copy(data, 0);
      toPubkey.copy(data, 32);
      data.writeBigUInt64LE(BigInt(1000000), 64);

      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [
        { name: 'from', type: 'pubkey' },
        { name: 'to', type: 'pubkey' },
        { name: 'amount', type: 'u64' },
      ];

      const result = decoder.decode(fields);
      expect(result.from).toBeInstanceOf(PublicKey);
      expect(result.to).toBeInstanceOf(PublicKey);
      expect(result.amount).toBe(BigInt(1000000));
      expect(result.from.toBuffer()).toEqual(fromPubkey);
      expect(result.to.toBuffer()).toEqual(toPubkey);
    });
  });

  describe('Error handling', () => {
    test('throws error for unsupported string type', () => {
      const data = Buffer.from([1, 2, 3]);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'text', type: 'string' }];
      expect(() => decoder.decode(fields)).toThrow(
        'String type not supported in C struct events'
      );
    });

    test('throws error for custom defined types', () => {
      const data = Buffer.from([1, 2, 3]);
      const decoder = new EventDecoder(data);
      const fields: IdlEventField[] = [{ name: 'custom', type: { defined: 'MyType' } }];
      expect(() => decoder.decode(fields)).toThrow('Custom types not yet supported');
    });
  });
});
