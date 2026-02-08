# Event System Usage Example

Complete example showing how to use events in a Solzempic program.

## Rust Program

```rust
use solzempic::{
    event, emit, AccountRefMut, Instruction, InstructionParams,
    SolzempicEntrypoint, params,
};
use solana_address::Address;
use pinocchio::ProgramResult;

// Define your events
#[event]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct InitializeEvent {
    pub authority: Address,
    pub initial_supply: u64,
}

// Define your program
#[SolzempicEntrypoint("Your11111111111111111111111111111111111111")]
pub enum MyProgram {
    Initialize = 0,
    Transfer = 1,
}

// Account definition
#[account(discriminator = 1)]
pub struct TokenAccount {
    pub discriminator: [u8; 8],
    pub owner: Address,
    pub balance: u64,
}

// Instruction parameters
#[params]
pub struct TransferParams {
    pub amount: u64,
}

// Transfer instruction
#[instruction]
pub struct Transfer<'a> {
    pub from: AccountRefMut<'a, TokenAccount>,
    pub to: AccountRefMut<'a, TokenAccount>,
    pub authority: Signer<'a>,
    pub clock: ClockSysvar<'a>,
}

#[instruction(TransferParams)]
impl<'a> Transfer<'a> {
    fn build(
        accounts: &'a [AccountView],
        _params: &TransferParams,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            from: AccountRefMut::load(&accounts[0])?,
            to: AccountRefMut::load(&accounts[1])?,
            authority: Signer::wrap(&accounts[2])?,
            clock: ClockSysvar::wrap(&accounts[3])?,
        })
    }

    fn validate(&self, _program_id: &Address, params: &TransferParams) -> ProgramResult {
        // Verify authority
        if self.from.get().owner != *self.authority.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Verify balance
        if self.from.get().balance < params.amount {
            return Err(ProgramError::InsufficientFunds);
        }

        Ok(())
    }

    fn execute(&mut self, _program_id: &Address, params: &TransferParams) -> ProgramResult {
        // Perform transfer
        self.from.get_mut().balance -= params.amount;
        self.to.get_mut().balance += params.amount;

        // Emit event
        emit!(TransferEvent {
            from: *self.from.key(),
            to: *self.to.key(),
            amount: params.amount,
            timestamp: self.clock.unix_timestamp,
        });

        Ok(())
    }
}
```

## Generate IDL

```bash
# Build with idl feature
cargo build --features idl

# Generate IDL
cargo test --features idl write_idl -- --ignored

# IDL will be written to: target/idl/your_program.json
```

## TypeScript Client

```typescript
import { createEventParser } from '@solzempic/events';
import {
  Connection,
  PublicKey,
  Transaction,
  SystemProgram,
} from '@solana/web3.js';
import idl from './idl.json';

// Setup
const connection = new Connection('https://api.devnet.solana.com');
const programId = new PublicKey('Your11111111111111111111111111111111111111');
const parser = createEventParser(idl);

// Example 1: Parse events from a transaction
async function parseTransactionEvents(signature: string) {
  const events = await parser.parseTransaction(connection, signature);

  for (const event of events) {
    if (event.name === 'TransferEvent') {
      console.log('Transfer detected:');
      console.log('  From:', event.data.from.toBase58());
      console.log('  To:', event.data.to.toBase58());
      console.log('  Amount:', event.data.amount.toString());
      console.log('  Timestamp:', new Date(Number(event.data.timestamp) * 1000));
      console.log('  Slot:', event.slot);
    } else if (event.name === 'InitializeEvent') {
      console.log('Initialize detected:');
      console.log('  Authority:', event.data.authority.toBase58());
      console.log('  Supply:', event.data.initial_supply.toString());
    }
  }
}

// Example 2: Subscribe to real-time events
async function subscribeToEvents() {
  const subscriptionId = await parser.subscribe(
    connection,
    programId,
    (event) => {
      console.log(`[${new Date().toISOString()}] Event: ${event.name}`);
      console.log('Data:', event.data);
      console.log('Signature:', event.signature);
    },
    {
      // Optional: filter for specific events
      eventNames: ['TransferEvent'],
    }
  );

  console.log('Subscribed to events. Subscription ID:', subscriptionId);

  // Unsubscribe after 60 seconds (example)
  setTimeout(async () => {
    await parser.unsubscribe(connection, subscriptionId);
    console.log('Unsubscribed from events');
  }, 60000);
}

// Example 3: Send transaction and parse its events
async function sendTransferAndParseEvents(
  payer: Keypair,
  from: PublicKey,
  to: PublicKey,
  amount: number
) {
  // Build transfer instruction
  const instruction = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: from, isSigner: false, isWritable: true },
      { pubkey: to, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([
      Buffer.from([1]), // Transfer discriminator
      Buffer.from(new BigUint64Array([BigInt(amount)]).buffer), // amount as u64
    ]),
  });

  // Send transaction
  const transaction = new Transaction().add(instruction);
  const signature = await sendAndConfirmTransaction(
    connection,
    transaction,
    [payer]
  );

  console.log('Transaction sent:', signature);

  // Wait for confirmation
  await connection.confirmTransaction(signature);

  // Parse events
  const events = await parser.parseTransaction(connection, signature);
  console.log('Events emitted:', events.length);

  return { signature, events };
}

// Example 4: Advanced filtering and processing
async function processEventsWithFiltering() {
  // Get recent transactions for program
  const signatures = await connection.getSignaturesForAddress(programId, {
    limit: 10,
  });

  for (const sig of signatures) {
    const events = await parser.parseTransaction(connection, sig.signature, {
      eventNames: ['TransferEvent'],
    });

    // Process only large transfers
    const largeTransfers = events.filter(
      (e) => e.name === 'TransferEvent' && e.data.amount > BigInt(1000000)
    );

    if (largeTransfers.length > 0) {
      console.log(`Large transfers in ${sig.signature}:`);
      for (const event of largeTransfers) {
        console.log(`  ${event.data.amount} tokens transferred`);
      }
    }
  }
}

// Run examples
(async () => {
  // Example usage
  await subscribeToEvents();

  // Parse a specific transaction
  const signature = 'your-transaction-signature';
  await parseTransactionEvents(signature);

  // Process recent events
  await processEventsWithFiltering();
})();
```

## Benefits

### Performance
- **Rust**: 0 CUs for serialization (vs ~200-500 CUs for Borsh)
- **TypeScript**: Instant discriminator lookup, efficient decoding

### Developer Experience
- **Type Safety**: Full TypeScript types from IDL
- **Real-Time**: Subscribe to events as they happen
- **Filtering**: Easy event filtering by name
- **Debugging**: Clear event structure in logs

### Production Ready
- **Battle Tested**: Comprehensive test coverage
- **Error Handling**: Graceful handling of malformed data
- **Scalable**: Efficient for high-throughput programs
