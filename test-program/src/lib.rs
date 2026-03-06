#![no_std]

extern crate alloc;

use bytemuck::{Pod, Zeroable};
use pinocchio::{error::ProgramError, AccountView, ProgramResult};
use solana_address::Address;
use solzempic::{
    emit, event::Event, transfer_lamports, Initializable, Instruction, InstructionParams, Loadable,
    Payer, Signer, SystemProgram, ValidatedAccount,
};
use solzempic_macros::SolzempicEntrypoint;

// ============================================================================
// Account Types
// ============================================================================

solzempic::define_account_types! {
    Counter = 1,
    Vault = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Counter {
    pub discriminator: [u8; 8],
    pub owner: Address,
    pub count: u64,
}

impl Loadable for Counter {
    const DISCRIMINATOR: u8 = AccountType::Counter as u8;
    const LEN: usize = core::mem::size_of::<Self>();
}

impl Initializable for Counter {}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct VaultAccount {
    pub discriminator: [u8; 8],
    pub authority: Address,
    pub total_deposited: u64,
}

impl Loadable for VaultAccount {
    const DISCRIMINATOR: u8 = AccountType::Vault as u8;
    const LEN: usize = core::mem::size_of::<Self>();
}

impl Initializable for VaultAccount {}

// ============================================================================
// Events
// ============================================================================

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CounterCreatedEvent {
    pub owner: Address,
    pub initial_count: u64,
}

impl Event for CounterCreatedEvent {
    const DISCRIMINATOR: [u8; 8] = [0xCC, 0x01, 0, 0, 0, 0, 0, 0];
    const NAME: &'static str = "CounterCreated";
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CounterIncrementedEvent {
    pub owner: Address,
    pub old_count: u64,
    pub new_count: u64,
}

impl Event for CounterIncrementedEvent {
    const DISCRIMINATOR: [u8; 8] = [0xCC, 0x02, 0, 0, 0, 0, 0, 0];
    const NAME: &'static str = "CounterIncremented";
}

// ============================================================================
// Instruction Parameters
// ============================================================================

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InitCounterParams {
    pub initial_count: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IncrementParams {
    pub amount: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TransferSolParams {
    pub amount: u64,
}

// ============================================================================
// Entrypoint + Dispatch (must come before instruction impls that use AccountRefMut)
// ============================================================================

#[SolzempicEntrypoint("SoLzeMPic1111111111111111111111111111111111")]
pub enum TestInstruction {
    InitCounter = 0,
    Increment = 1,
    TransferSol = 2,
}

// ============================================================================
// Instructions
// ============================================================================

// --- InitCounter ---
pub struct InitCounter<'a> {
    pub counter: AccountRefMut<'a, Counter>,
    pub payer: Payer<'a>,
    pub system_program: SystemProgram<'a>,
}

impl InstructionParams for InitCounter<'_> {
    type Params = InitCounterParams;
}

impl<'a> Instruction<'a> for InitCounter<'a> {
    fn build(accounts: &'a [AccountView], _params: &Self::Params) -> Result<Self, ProgramError> {
        let payer = Payer::wrap(&accounts[0])?;
        let system_program = SystemProgram::wrap(&accounts[2])?;
        let counter = AccountRefMut::<Counter>::init(&accounts[1])?;

        Ok(Self {
            counter,
            payer,
            system_program,
        })
    }

    fn validate(&self, _program_id: &Address, _params: &Self::Params) -> ProgramResult {
        Ok(())
    }

    fn execute(&mut self, _program_id: &Address, params: &Self::Params) -> ProgramResult {
        let data = self.counter.get_mut();
        data.owner = *self.payer.address();
        data.count = params.initial_count;

        emit!(CounterCreatedEvent {
            owner: *self.payer.address(),
            initial_count: params.initial_count,
        });

        Ok(())
    }
}

// --- Increment ---
pub struct Increment<'a> {
    pub counter: AccountRefMut<'a, Counter>,
    pub owner: Signer<'a>,
}

impl InstructionParams for Increment<'_> {
    type Params = IncrementParams;
}

impl<'a> Instruction<'a> for Increment<'a> {
    fn build(accounts: &'a [AccountView], _params: &Self::Params) -> Result<Self, ProgramError> {
        Ok(Self {
            counter: AccountRefMut::load(&accounts[0])?,
            owner: Signer::wrap(&accounts[1])?,
        })
    }

    fn validate(&self, _program_id: &Address, _params: &Self::Params) -> ProgramResult {
        if self.counter.get().owner != *self.owner.address() {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(())
    }

    fn execute(&mut self, _program_id: &Address, params: &Self::Params) -> ProgramResult {
        let data = self.counter.get_mut();
        let old_count = data.count;
        data.count = old_count
            .checked_add(params.amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        emit!(CounterIncrementedEvent {
            owner: data.owner,
            old_count,
            new_count: data.count,
        });

        Ok(())
    }
}

// --- TransferSol ---
pub struct TransferSol<'a> {
    pub from: Signer<'a>,
    pub to: &'a AccountView,
    pub system_program: SystemProgram<'a>,
}

impl InstructionParams for TransferSol<'_> {
    type Params = TransferSolParams;
}

impl<'a> Instruction<'a> for TransferSol<'a> {
    fn build(accounts: &'a [AccountView], _params: &Self::Params) -> Result<Self, ProgramError> {
        Ok(Self {
            from: Signer::wrap(&accounts[0])?,
            to: &accounts[1],
            system_program: SystemProgram::wrap(&accounts[2])?,
        })
    }

    fn validate(&self, _program_id: &Address, params: &Self::Params) -> ProgramResult {
        if params.amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(())
    }

    fn execute(&mut self, _program_id: &Address, params: &Self::Params) -> ProgramResult {
        transfer_lamports(
            self.from.info,
            self.to,
            self.system_program.info(),
            params.amount,
        )
    }
}
