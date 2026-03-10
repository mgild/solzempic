/**
 * Event parser for extracting and decoding events from Solana transaction logs
 */

import { Connection, ParsedTransactionWithMeta, PublicKey } from '@solana/web3.js';
import * as bs58 from 'bs58';
import { Buffer } from 'buffer';
import { EventDecoder } from './decoder';
import {
  SolzempicIdl,
  ParsedEvent,
  EventWithMetadata,
  ParserOptions,
  IdlEvent,
} from './types';

/**
 * Event parser for Solzempic programs
 */
export class EventParser {
  private eventsByDiscriminator: Map<string, IdlEvent>;

  constructor(private idl: SolzempicIdl) {
    // Build discriminator index for fast lookup
    this.eventsByDiscriminator = new Map();
    for (const event of idl.events) {
      const discKey = Buffer.from(event.discriminator).toString('hex');
      this.eventsByDiscriminator.set(discKey, event);
    }
  }

  /**
   * Parse events from transaction logs
   *
   * @param logs - Transaction logs (base64-encoded)
   * @param options - Parser options
   * @returns Parsed events
   */
  parseLogs(logs: string[], options?: ParserOptions): ParsedEvent[] {
    const events: ParsedEvent[] = [];
    const eventNames = new Set(options?.eventNames);

    for (const log of logs) {
      // Solana logs events as "Program data: <base64>"
      if (!log.startsWith('Program data: ')) {
        continue;
      }

      const base64Data = log.slice('Program data: '.length);
      try {
        const data = Buffer.from(base64Data, 'base64');

        // Events have 8-byte discriminator + event data
        if (data.length < 8) {
          continue;
        }

        // Extract discriminator
        const discriminator = data.subarray(0, 8);
        const discKey = discriminator.toString('hex');

        // Lookup event by discriminator
        const eventDef = this.eventsByDiscriminator.get(discKey);
        if (!eventDef) {
          continue;
        }

        // Filter by name if specified
        if (eventNames.size > 0 && !eventNames.has(eventDef.name)) {
          continue;
        }

        // Decode event data
        const eventData = data.subarray(8);
        const decoder = new EventDecoder(eventData);
        const decoded = decoder.decode(eventDef.fields);

        events.push({
          name: eventDef.name,
          data: decoded,
        });
      } catch (err) {
        // Malformed event payloads are expected in mixed log streams.
        // Keep unknown parser faults visible, but suppress noisy bounds errors.
        const msg = err instanceof Error ? err.message : String(err);
        const isBoundsError =
          err instanceof RangeError ||
          /outside buffer bounds|out of range/i.test(msg);
        if (!isBoundsError) {
          console.warn('Failed to parse event log:', err);
        }
      }
    }

    return events;
  }

  /**
   * Parse events from a transaction signature
   *
   * @param connection - Solana connection
   * @param signature - Transaction signature
   * @param options - Parser options
   * @returns Parsed events with metadata
   */
  async parseTransaction(
    connection: Connection,
    signature: string,
    options?: ParserOptions
  ): Promise<EventWithMetadata[]> {
    const tx = await connection.getParsedTransaction(signature, {
      maxSupportedTransactionVersion: 0,
    });

    if (!tx || !tx.meta) {
      return [];
    }

    const logs = tx.meta.logMessages || [];
    const events = this.parseLogs(logs, options);

    // Add metadata
    return events.map((event) => ({
      ...event,
      signature,
      slot: tx.slot,
      blockTime: tx.blockTime ?? null,
    }));
  }

  /**
   * Subscribe to events in real-time
   *
   * @param connection - Solana connection
   * @param programId - Program public key
   * @param callback - Callback for each event
   * @param options - Parser options
   * @returns Subscription ID (for unsubscribing)
   */
  async subscribe(
    connection: Connection,
    programId: PublicKey,
    callback: (event: EventWithMetadata) => void,
    options?: ParserOptions
  ): Promise<number> {
    return connection.onLogs(
      programId,
      async (logs, ctx) => {
        const events = this.parseLogs(logs.logs, options);

        for (const event of events) {
          callback({
            ...event,
            signature: logs.signature,
            slot: ctx.slot,
            blockTime: null, // Real-time logs don't have blockTime
          });
        }
      },
      'confirmed'
    );
  }

  /**
   * Unsubscribe from event stream
   *
   * @param connection - Solana connection
   * @param subscriptionId - Subscription ID from subscribe()
   */
  async unsubscribe(connection: Connection, subscriptionId: number): Promise<void> {
    await connection.removeOnLogsListener(subscriptionId);
  }
}

/**
 * Create an event parser from IDL
 *
 * @param idl - Solzempic program IDL
 * @returns Event parser instance
 */
export function createEventParser(idl: SolzempicIdl): EventParser {
  return new EventParser(idl);
}
