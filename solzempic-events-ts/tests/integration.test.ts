/**
 * Integration tests for complete event parsing workflow
 */

import { createEventParser, EventParser } from '../src/parser';
import { SolzempicIdl } from '../src/types';
import { Buffer } from 'buffer';

describe('Integration Tests', () => {
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
        name: 'InitializeEvent',
        discriminator: [10, 20, 30, 40, 50, 60, 70, 80],
        fields: [
          { name: 'authority', type: 'pubkey' },
          { name: 'supply', type: 'u64' },
        ],
      },
    ],
    errors: [],
  };

  describe('End-to-End Event Parsing', () => {
    test('parses complete transaction with multiple events', () => {
      const parser = createEventParser(mockIdl);

      // Create TransferEvent
      const transferEvent = Buffer.alloc(72);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(transferEvent, 0);
      transferEvent.writeBigUInt64LE(BigInt(1000), 64);

      // Create InitializeEvent
      const initEvent = Buffer.alloc(40);
      Buffer.from([10, 20, 30, 40, 50, 60, 70, 80]).copy(initEvent, 0);
      initEvent.writeBigUInt64LE(BigInt(1000000), 32);

      const logs = [
        'Program invoke: MyProgram',
        `Program data: ${transferEvent.toString('base64')}`,
        'Program log: Transfer completed',
        `Program data: ${initEvent.toString('base64')}`,
        'Program return: success',
      ];

      const events = parser.parseLogs(logs);

      expect(events).toHaveLength(2);
      expect(events[0].name).toBe('TransferEvent');
      expect(events[0].data.amount).toBe(BigInt(1000));
      expect(events[1].name).toBe('InitializeEvent');
      expect(events[1].data.supply).toBe(BigInt(1000000));
    });

    test('handles mixed valid and invalid logs', () => {
      const parser = createEventParser(mockIdl);

      const validEvent = Buffer.alloc(72);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(validEvent, 0);

      const invalidData = Buffer.from([99, 99, 99]); // Unknown discriminator

      const logs = [
        `Program data: ${validEvent.toString('base64')}`,
        `Program data: ${invalidData.toString('base64')}`,
        'Program data: invalid-base64!!!',
        'Program log: Regular log message',
      ];

      const events = parser.parseLogs(logs);

      // Should only parse the valid event
      expect(events).toHaveLength(1);
      expect(events[0].name).toBe('TransferEvent');
    });

    test('filters events by name correctly', () => {
      const parser = createEventParser(mockIdl);

      const transferEvent = Buffer.alloc(72);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(transferEvent, 0);

      const initEvent = Buffer.alloc(40);
      Buffer.from([10, 20, 30, 40, 50, 60, 70, 80]).copy(initEvent, 0);

      const logs = [
        `Program data: ${transferEvent.toString('base64')}`,
        `Program data: ${initEvent.toString('base64')}`,
      ];

      // Filter for only TransferEvent
      const transferEvents = parser.parseLogs(logs, {
        eventNames: ['TransferEvent'],
      });

      expect(transferEvents).toHaveLength(1);
      expect(transferEvents[0].name).toBe('TransferEvent');

      // Filter for only InitializeEvent
      const initEvents = parser.parseLogs(logs, {
        eventNames: ['InitializeEvent'],
      });

      expect(initEvents).toHaveLength(1);
      expect(initEvents[0].name).toBe('InitializeEvent');

      // Filter for both
      const allEvents = parser.parseLogs(logs, {
        eventNames: ['TransferEvent', 'InitializeEvent'],
      });

      expect(allEvents).toHaveLength(2);
    });
  });

  describe('Parser Factory', () => {
    test('createEventParser creates valid parser', () => {
      const parser = createEventParser(mockIdl);
      expect(parser).toBeInstanceOf(EventParser);
    });

    test('parser works with empty IDL events', () => {
      const emptyIdl: SolzempicIdl = {
        ...mockIdl,
        events: [],
      };

      const parser = createEventParser(emptyIdl);
      const logs = ['Program data: AQIDBAUG', 'Program log: test'];

      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(0);
    });
  });

  describe('Real-world Scenarios', () => {
    test('handles high-frequency event stream', () => {
      const parser = createEventParser(mockIdl);

      // Simulate 100 events in a single transaction
      const logs: string[] = [];
      for (let i = 0; i < 100; i++) {
        const event = Buffer.alloc(72);
        Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(event, 0);
        event.writeBigUInt64LE(BigInt(i), 64);
        logs.push(`Program data: ${event.toString('base64')}`);
      }

      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(100);

      // Verify all amounts are correct
      for (let i = 0; i < 100; i++) {
        expect(events[i].data.amount).toBe(BigInt(i));
      }
    });

    test('handles events with all zeros', () => {
      const parser = createEventParser(mockIdl);

      const event = Buffer.alloc(72);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(event, 0);
      // Rest is zeros

      const logs = [`Program data: ${event.toString('base64')}`];
      const events = parser.parseLogs(logs);

      expect(events).toHaveLength(1);
      expect(events[0].data.amount).toBe(BigInt(0));
    });

    test('handles events with maximum values', () => {
      const parser = createEventParser(mockIdl);

      const event = Buffer.alloc(72);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(event, 0);
      event.writeBigUInt64LE(BigInt('18446744073709551615'), 64); // Max u64

      const logs = [`Program data: ${event.toString('base64')}`];
      const events = parser.parseLogs(logs);

      expect(events).toHaveLength(1);
      expect(events[0].data.amount).toBe(BigInt('18446744073709551615'));
    });
  });

  describe('Error Resilience', () => {
    test('continues parsing after encountering bad event', () => {
      const parser = createEventParser(mockIdl);

      const goodEvent = Buffer.alloc(72);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(goodEvent, 0);
      goodEvent.writeBigUInt64LE(BigInt(100), 64);

      const badEvent = Buffer.alloc(10); // Too short
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(badEvent, 0);

      const logs = [
        `Program data: ${goodEvent.toString('base64')}`,
        `Program data: ${badEvent.toString('base64')}`,
        `Program data: ${goodEvent.toString('base64')}`,
      ];

      const events = parser.parseLogs(logs);

      // Should skip bad event and continue
      expect(events).toHaveLength(2);
      expect(events[0].data.amount).toBe(BigInt(100));
      expect(events[1].data.amount).toBe(BigInt(100));
    });

    test('handles corrupt base64 gracefully', () => {
      const parser = createEventParser(mockIdl);

      const logs = [
        'Program data: !!!invalid!!!',
        'Program data: @#$%^&*()',
        'Program data: ',
      ];

      // Should not throw, just return empty
      const events = parser.parseLogs(logs);
      expect(events).toHaveLength(0);
    });

    test('handles empty log array', () => {
      const parser = createEventParser(mockIdl);
      const events = parser.parseLogs([]);
      expect(events).toHaveLength(0);
    });
  });

  describe('Performance', () => {
    test('discriminator lookup is O(1)', () => {
      const parser = createEventParser(mockIdl);

      const event = Buffer.alloc(72);
      Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).copy(event, 0);

      const logs = [`Program data: ${event.toString('base64')}`];

      // First parse
      const start1 = Date.now();
      parser.parseLogs(logs);
      const time1 = Date.now() - start1;

      // Repeat 1000 times - should be similar time due to O(1) lookup
      const start2 = Date.now();
      for (let i = 0; i < 1000; i++) {
        parser.parseLogs(logs);
      }
      const time2 = Date.now() - start2;

      // Average time should be roughly the same (within 10x)
      const avgTime = time2 / 1000;
      expect(avgTime).toBeLessThan(time1 * 10);
    });
  });
});
