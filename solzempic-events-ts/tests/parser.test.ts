/**
 * Tests for EventParser
 */

import { EventParser } from '../src/parser';
import { SolzempicIdl } from '../src/types';
import { Buffer } from 'buffer';

describe('EventParser', () => {
  // Mock IDL with test events
  const mockIdl: SolzempicIdl = {
    address: 'Test1111111111111111111111111111111111111',
    metadata: {
      name: 'test_program',
      version: '0.1.0',
      spec: '0.1.0',
    },
    instructions: [],
    accounts: [],
    types: [],
    events: [
      {
        name: 'TransferEvent',
        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
        fields: [
          { name: 'from', type: 'pubkey' },
          { name: 'to', type: 'pubkey' },
          { name: 'amount', type: 'u64' },
        ],
      },
      {
        name: 'SwapEvent',
        discriminator: [10, 20, 30, 40, 50, 60, 70, 80],
        fields: [
          { name: 'tokenIn', type: 'pubkey' },
          { name: 'tokenOut', type: 'pubkey' },
          { name: 'amountIn', type: 'u64' },
          { name: 'amountOut', type: 'u64' },
        ],
      },
    ],
    errors: [],
  };

  describe('Constructor', () => {
    test('creates parser from IDL', () => {
      const parser = new EventParser(mockIdl);
      expect(parser).toBeInstanceOf(EventParser);
    });

    test('builds discriminator index', () => {
      const parser = new EventParser(mockIdl);
      // Discriminator index should be built (tested indirectly via parsing)
      expect(parser).toBeTruthy();
    });
  });

  describe('parseLogs', () => {
    test('parses valid event log', () => {
      const parser = new EventParser(mockIdl);

      // Create event data: discriminator + event struct
      const eventData = Buffer.alloc(80); // 8 + 32 + 32 + 8
      eventData.writeUInt8(1, 0); // discriminator[0]
      eventData.writeUInt8(2, 1); // discriminator[1]
      eventData.writeUInt8(3, 2); // discriminator[2]
      eventData.writeUInt8(4, 3); // discriminator[3]
      eventData.writeUInt8(5, 4); // discriminator[4]
      eventData.writeUInt8(6, 5); // discriminator[5]
      eventData.writeUInt8(7, 6); // discriminator[6]
      eventData.writeUInt8(8, 7); // discriminator[7]

      // Write pubkeys and amount (all zeros for simplicity)
      eventData.writeBigUInt64LE(BigInt(1000), 72);

      const base64Data = eventData.toString('base64');
      const logs = [`Program data: ${base64Data}`];

      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(1);
      expect(events[0].name).toBe('TransferEvent');
      expect(events[0].data.amount).toBe(BigInt(1000));
    });

    test('ignores non-event logs', () => {
      const parser = new EventParser(mockIdl);
      const logs = [
        'Program log: Some message',
        'Program invoke: SomeProgram',
        'Program return: success',
      ];

      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(0);
    });

    test('ignores logs with unknown discriminator', () => {
      const parser = new EventParser(mockIdl);

      // Create event with unknown discriminator
      const eventData = Buffer.alloc(80);
      eventData.writeUInt8(99, 0); // Unknown discriminator

      const base64Data = eventData.toString('base64');
      const logs = [`Program data: ${base64Data}`];

      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(0);
    });

    test('parses multiple events from logs', () => {
      const parser = new EventParser(mockIdl);

      // Create two events
      const event1 = Buffer.alloc(80);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(event1, 0);
      event1.writeBigUInt64LE(BigInt(100), 72);

      const event2 = Buffer.alloc(88);
      Buffer.from([10, 20, 30, 40, 50, 60, 70, 80]).copy(event2, 0);
      event2.writeBigUInt64LE(BigInt(200), 80);

      const logs = [
        `Program data: ${event1.toString('base64')}`,
        'Program log: Some message',
        `Program data: ${event2.toString('base64')}`,
      ];

      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(2);
      expect(events[0].name).toBe('TransferEvent');
      expect(events[0].data.amount).toBe(BigInt(100));
      expect(events[1].name).toBe('SwapEvent');
      expect(events[1].data.amountOut).toBe(BigInt(200));
    });

    test('filters events by name', () => {
      const parser = new EventParser(mockIdl);

      const event1 = Buffer.alloc(80);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(event1, 0);

      const event2 = Buffer.alloc(88);
      Buffer.from([10, 20, 30, 40, 50, 60, 70, 80]).copy(event2, 0);

      const logs = [
        `Program data: ${event1.toString('base64')}`,
        `Program data: ${event2.toString('base64')}`,
      ];

      const events = parser.parseLogs(logs, { eventNames: ['TransferEvent'] });
      expect(events).toHaveLength(1);
      expect(events[0].name).toBe('TransferEvent');
    });

    test('handles malformed event data gracefully', () => {
      const parser = new EventParser(mockIdl);

      // Malformed data (too short)
      const eventData = Buffer.from([1, 2, 3]);
      const logs = [`Program data: ${eventData.toString('base64')}`];

      // Should not throw, just skip malformed events
      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(0);
    });

    test('handles invalid base64 gracefully', () => {
      const parser = new EventParser(mockIdl);
      const logs = ['Program data: !!!invalid-base64!!!'];

      // Should not throw
      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(0);
    });
  });

  describe('Event discriminator matching', () => {
    test('correctly matches 8-byte discriminator', () => {
      const parser = new EventParser(mockIdl);

      // Full 8-byte discriminator match
      const eventData = Buffer.alloc(80);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(eventData, 0);

      const logs = [`Program data: ${eventData.toString('base64')}`];
      const events = parser.parseLogs(logs);

      expect(events).toHaveLength(1);
      expect(events[0].name).toBe('TransferEvent');
    });

    test('rejects partial discriminator match', () => {
      const parser = new EventParser(mockIdl);

      // Discriminator that matches first byte but not others
      const eventData = Buffer.alloc(80);
      Buffer.from([1, 0, 0, 0, 0, 0, 0, 0]).copy(eventData, 0);

      const logs = [`Program data: ${eventData.toString('base64')}`];
      const events = parser.parseLogs(logs);

      expect(events).toHaveLength(0);
    });
  });
});
