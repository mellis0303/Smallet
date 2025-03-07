//! This program can be used to allow a smallet to govern anything a regular
//! [Pubkey] can govern. One can use the smallet as a BPF program upgrade
//! authority, a mint authority, etc.
//!
//! To use, one must first create a [Smallet] account, specifying two important
//! parameters:
//!
//! 1. Owners - the set of addresses that sign transactions for the smallet.
//! 2. Threshold - the number of signers required to execute a transaction.
//! 3. Minimum Delay - the minimum amount of time that must pass before a [Transaction]
//!                    can be executed. If 0, this is ignored.
//!
//! Once the [Smallet] account is created, one can create a [Transaction]
//! account, specifying the parameters for a normal Solana instruction.
//!
//! To sign, owners should invoke the [smallet::approve] instruction, and finally,
//! [smallet::execute_transaction], once enough (i.e. [Smallet::threshold]) of the owners have
//! signed.

#![allow(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]
#![allow(clippy::unwrap_used)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use vipers::prelude::*;

mod events;
mod instructions;
mod state;
mod validators;

pub use events::*;
pub use instructions::*;
pub use state::*;

// Number of seconds in a day
pub const SECONDS_PER_DAY: i64 = 60 * 60 * 24;

// Maximum timelock delay.
pub const MAX_DELAY_SECONDS: i64 = 365 * SECONDS_PER_DAY;

// Default number of seconds until a transaction expires.
pub const DEFAULT_GRACE_PERIOD: i64 = 14 * SECONDS_PER_DAY;

// Constant declaring that there is no ETA of the transaction
pub const NO_ETA: i64 = -1;

declare_id!("7vZw152zk65W5F59S6Svt29JjDhzZocWvF7PYvjNCAKB");

#[program]
// Smallet program
pub mod smallet {
    use super::*;
    
	// Initializes a new [Smallet] account with a set of owners and a threshold
    #[access_control(ctx.accounts.validate())]
    pub fn create_smallet(
        ctx: Context<CreateSmallet>,
        _bump: u8,
        max_owners: u8,
        owners: Vec<Pubkey>,
        threshold: u64,
        minimum_delay: i64,
    ) -> Result<()> {
        invariant!(minimum_delay >= 0, "delay must be positive");
        invariant!(minimum_delay < MAX_DELAY_SECONDS, DelayTooHigh);

        invariant!((max_owners as usize) >= owners.len(), "max_owners");

        let smallet = &mut ctx.accounts.smallet;
        smallet.base = ctx.accounts.base.key();
        smallet.bump = *unwrap_int!(ctx.bumps.get("smallet"));

        smallet.threshold = threshold;
        smallet.minimum_delay = minimum_delay;
        smallet.grace_period = DEFAULT_GRACE_PERIOD;

        smallet.owner_set_seqno = 0;
        smallet.num_transactions = 0;

        smallet.owners = owners.clone();

        emit!(WalletCreateEvent {
            smallet: ctx.accounts.smallet.key(),
            owners,
            threshold,
            minimum_delay,
            timestamp: Clock::get()?.unix_timestamp
        });
        Ok(())
    }

	// Sets the owners field on the smallet. The only way this can be invoked 
	// is via a recursive call from execute_transaction -> set_owners.
    #[access_control(ctx.accounts.validate())]
    pub fn set_owners(ctx: Context<Auth>, owners: Vec<Pubkey>) -> Result<()> {
        let smallet = &mut ctx.accounts.smallet;
        if (owners.len() as u64) < smallet.threshold {
            smallet.threshold = owners.len() as u64;
        }

        smallet.owners = owners.clone();
        smallet.owner_set_seqno = unwrap_int!(smallet.owner_set_seqno.checked_add(1));

        emit!(WalletSetOwnersEvent {
            smallet: ctx.accounts.smallet.key(),
            owners,
            timestamp: Clock::get()?.unix_timestamp
        });
        Ok(())
    }

	// Changes the execution threshold of the smallet. The only way this can be 
	// invoked is via a recursive call from execute_transaction ->
	// change_threshold.
    #[access_control(ctx.accounts.validate())]
    pub fn change_threshold(ctx: Context<Auth>, threshold: u64) -> Result<()> {
        invariant!(
            threshold <= ctx.accounts.smallet.owners.len() as u64,
            InvalidThreshold
        );
        let smallet = &mut ctx.accounts.smallet;
        smallet.threshold = threshold;

        emit!(WalletChangeThresholdEvent {
            smallet: ctx.accounts.smallet.key(),
            threshold,
            timestamp: Clock::get()?.unix_timestamp
        });
        Ok(())
    }

	// Creates a new [Transaction] account, automatically signed by the creator, 
	// which must be one of the owners of the smallet.
    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        bump: u8,
        instructions: Vec<TXInstruction>,
    ) -> Result<()> {
        create_transaction_with_timelock(ctx, bump, instructions, NO_ETA)
    }

	// Creates a new [Transaction] account with time delay.
    #[access_control(ctx.accounts.validate())]
    pub fn create_transaction_with_timelock(
        ctx: Context<CreateTransaction>,
        _bump: u8,
        instructions: Vec<TXInstruction>,
        eta: i64,
    ) -> Result<()> {
        let smallet = &ctx.accounts.smallet;
        let owner_index = smallet.try_owner_index(ctx.accounts.proposer.key())?;

        let clock = Clock::get()?;
        let current_ts = clock.unix_timestamp;
        if smallet.minimum_delay != 0 {
            invariant!(
                eta >= unwrap_int!(current_ts.checked_add(smallet.minimum_delay as i64)),
                InvalidETA
            );
        }
        if eta != NO_ETA {
            invariant!(eta >= 0, "ETA must be positive");
            let delay = unwrap_int!(eta.checked_sub(current_ts));
            invariant!(delay >= 0, "ETA must be in the future");
            invariant!(delay <= MAX_DELAY_SECONDS, DelayTooHigh);
        }

		// generate the signers boolean list
        let owners = &smallet.owners;
        let mut signers = Vec::new();
        signers.resize(owners.len(), false);
        signers[owner_index] = true;

        let index = smallet.num_transactions;
        let smallet = &mut ctx.accounts.smallet;
        smallet.num_transactions = unwrap_int!(smallet.num_transactions.checked_add(1));

		// init the TX
        let tx = &mut ctx.accounts.transaction;
        tx.smallet = smallet.key();
        tx.index = index;
        tx.bump = *unwrap_int!(ctx.bumps.get("transaction"));

        tx.proposer = ctx.accounts.proposer.key();
        tx.instructions = instructions.clone();
        tx.signers = signers;
        tx.owner_set_seqno = smallet.owner_set_seqno;
        tx.eta = eta;

        tx.executor = Pubkey::default();
        tx.executed_at = -1;

        emit!(TransactionCreateEvent {
            smallet: ctx.accounts.smallet.key(),
            transaction: ctx.accounts.transaction.key(),
            proposer: ctx.accounts.proposer.key(),
            instructions,
            eta,
            timestamp: Clock::get()?.unix_timestamp
        });
        Ok(())
    }

	// Approves a transaction on behalf of an owner of the [Smallet]
    #[access_control(ctx.accounts.validate())]
    pub fn approve(ctx: Context<Approve>) -> Result<()> {
        instructions::approve::handler(ctx)
    }

	// Unapproves a transaction on behald of an owner of the [Smallet]
    #[access_control(ctx.accounts.validate())]
    pub fn unapprove(ctx: Context<Approve>) -> Result<()> {
        instructions::unapprove::handler(ctx)
    }

	// Executes the given transaction if threshold owners have signed it.
    #[access_control(ctx.accounts.validate())]
    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        let smallet = &ctx.accounts.smallet;
        let wallet_seeds: &[&[&[u8]]] = &[&[
            b"CosmicSmallet" as &[u8],
            &smallet.base.to_bytes(),
            &[smallet.bump],
        ]];
        do_execute_transaction(ctx, wallet_seeds)
    }

	// Executes the given transaction signed by the given derived address,
    // if threshold owners have signed it.
    // This allows a Smallet to receive SOL.
    #[access_control(ctx.accounts.validate())]
    pub fn execute_transaction_derived(
        ctx: Context<ExecuteTransaction>,
        index: u64,
        bump: u8,
    ) -> Result<()> {
        let smallet = &ctx.accounts.smallet;
        // Execute the transaction signed by the smallet.
        let wallet_seeds: &[&[&[u8]]] = &[&[
            b"CosmicSmalletDerived" as &[u8],
            &smallet.key().to_bytes(),
            &index.to_le_bytes(),
            &[bump],
        ]];
        do_execute_transaction(ctx, wallet_seeds)
    }

	// Invokes an arbitrary instruction as a PDA derived from the owner,
    // i.e. as an "Owner Invoker".
    // This is useful for using the multisig as a whitelist or as a council,
    // e.g. a whitelist of approved owners.
    #[access_control(ctx.accounts.validate())]
    pub fn owner_invoke_instruction(
        ctx: Context<OwnerInvokeInstruction>,
        index: u64,
        bump: u8,
        ix: TXInstruction,
    ) -> Result<()> {
        let smallet = &ctx.accounts.smallet;
        let invoker_seeds: &[&[&[u8]]] = &[&[
            b"CosmicSmalletOwnerInvoker" as &[u8],
            &smallet.key().to_bytes(),
            &index.to_le_bytes(),
            &[bump],
        ]];

        solana_program::program::invoke_signed(
            &(&ix).into(),
            ctx.remaining_accounts,
            invoker_seeds,
        )?;

        Ok(())
    }

	// Invokes an arbitrary instruction as a PDA derived from the owner,
    // i.e. as an "Owner Invoker".
    //
    // This is useful for using the multisig as a whitelist or as a council,
    // e.g. a whitelist of approved owners.
    //
    // # Arguments
    // - `index` - The index of the owner-invoker.
    // - `bump` - Bump seed of the owner-invoker.
    // - `invoker` - The owner-invoker.
    // - `data` - The raw bytes of the instruction data.
    #[access_control(ctx.accounts.validate())]
    pub fn owner_invoke_instruction_v2(
        ctx: Context<OwnerInvokeInstruction>,
        index: u64,
        bump: u8,
        invoker: Pubkey,
        data: Vec<u8>,
    ) -> Result<()> {
        let smallet = &ctx.accounts.smallet;
        // Execute the transaction signed by the smallet.
        let invoker_seeds: &[&[&[u8]]] = &[&[
            b"CosmicSmalletOwnerInvoker" as &[u8],
            &smallet.key().to_bytes(),
            &index.to_le_bytes(),
            &[bump],
        ]];

        let program_id = ctx.remaining_accounts[0].key();
        let accounts: Vec<AccountMeta> = ctx.remaining_accounts[1..]
            .iter()
            .map(|v| AccountMeta {
                pubkey: *v.key,
                is_signer: if v.key == &invoker { true } else { v.is_signer },
                is_writable: v.is_writable,
            })
            .collect();
        let ix = &solana_program::instruction::Instruction {
            program_id,
            accounts,
            data,
        };

        solana_program::program::invoke_signed(ix, ctx.remaining_accounts, invoker_seeds)?;
        Ok(())
    }

	// Creates a struct containing a reverse mapping of a subaccount to a
    // [Smallet].
    #[access_control(ctx.accounts.validate())]
    pub fn create_subaccount_info(
        ctx: Context<CreateSubaccountInfo>,
        _bump: u8,
        subaccount: Pubkey,
        smallet: Pubkey,
        index: u64,
        subaccount_type: SubaccountType,
    ) -> Result<()> {
        let (address, _derived_bump) = match subaccount_type {
            SubaccountType::Derived => Pubkey::find_program_address(
                &[
                    b"CosmicSmalletDerived" as &[u8],
                    &smallet.to_bytes(),
                    &index.to_le_bytes(),
                ],
                &crate::ID,
            ),
            SubaccountType::OwnerInvoker => Pubkey::find_program_address(
                &[
                    b"CosmicSmalletOwnerInvoker" as &[u8],
                    &smallet.to_bytes(),
                    &index.to_le_bytes(),
                ],
                &crate::ID,
            ),
        };

        invariant!(address == subaccount, SubaccountOwnerMismatch);

        let info = &mut ctx.accounts.subaccount_info;
        info.smallet = smallet;
        info.subaccount_type = subaccount_type;
        info.index = index;

        Ok(())
    }
}
// Accounts for [smallet::create_smallet].
#[derive(Accounts)]
#[instruction(bump: u8, max_owners: u8)]
pub struct CreateSmallet<'info> {
	// Base key of the Smallet.
    pub base: Signer<'info>,
	// The [Smallet] to create.
    #[account(
        init,
        seeds = [
            b"CosmicSmallet".as_ref(),
            base.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = Smallet::space(max_owners),
    )]
    pub smallet: Account<'info, Smallet>,
	// Payer to create the smallet.
    #[account(mut)]
    pub payer: Signer<'info>,
	// The [System] program.
    pub system_program: Program<'info, System>,
}
// Accounts for [smallet::set_owners] and [smallet::change_threshold].
#[derive(Accounts)]
pub struct Auth<'info> {
	// The [Smallet]
    #[account(mut, signer)]
    pub smallet: Account<'info, Smallet>,
}
// Accounts for [smallet::create_transaction].
#[derive(Accounts)]
#[instruction(bump: u8, instructions: Vec<TXInstruction>)]
pub struct CreateTransaction<'info> {
	// The [Smallet]
    #[account(mut)]
    pub smallet: Account<'info, Smallet>,
    // The [Transaction]
    #[account(
        init,
        seeds = [
            b"CosmicTransaction".as_ref(),
            smallet.key().to_bytes().as_ref(),
            smallet.num_transactions.to_le_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = Transaction::space(instructions),
    )]
    pub transaction: Account<'info, Transaction>,
    // One of the owners. Checked in the handler via [Smallet::try_owner_index].
    pub proposer: Signer<'info>,
    // Payer to create the [Transaction].
    #[account(mut)]
    pub payer: Signer<'info>,
    // The [System] program.
    pub system_program: Program<'info, System>,
}

// Accounts for [smallet::execute_transaction].
#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
	// The [Smallet].
    pub smallet: Account<'info, Smallet>,
    // The [Transaction] to execute.
    #[account(mut)]
    pub transaction: Account<'info, Transaction>,
    // An owner of the [Smallet].
    pub owner: Signer<'info>,
}

// Accounts for [smallet::owner_invoke_instruction].
#[derive(Accounts)]
pub struct OwnerInvokeInstruction<'info> {
	// The [Smallet]
    pub smallet: Account<'info, Smallet>,
    // An owner of the [Smallet].
    pub owner: Signer<'info>,
}

// Accounts for [smallet::create_subaccount_info].
#[derive(Accounts)]
#[instruction(bump: u8, subaccount: Pubkey)]
pub struct CreateSubaccountInfo<'info> {
	// The [SubaccountInfo] to create.
    #[account(
        init,
        seeds = [
            b"CosmicSubaccountInfo".as_ref(),
            &subaccount.to_bytes()
        ],
        bump,
        payer = payer,
        space = 8 + SubaccountInfo::LEN
    )]
    pub subaccount_info: Account<'info, SubaccountInfo>,
    // Payer to create the [SubaccountInfo].
    #[account(mut)]
    pub payer: Signer<'info>,
    // The [System] program.
    pub system_program: Program<'info, System>,
}

fn do_execute_transaction(ctx: Context<ExecuteTransaction>, seeds: &[&[&[u8]]]) -> Result<()> {
    for ix in ctx.accounts.transaction.instructions.iter() {
        solana_program::program::invoke_signed(&(ix).into(), ctx.remaining_accounts, seeds)?;
    }
	
	// Burn the transaction to ensure one time use.
    let tx = &mut ctx.accounts.transaction;
    tx.executor = ctx.accounts.owner.key();
    tx.executed_at = Clock::get()?.unix_timestamp;

    emit!(TransactionExecuteEvent {
        smallet: ctx.accounts.smallet.key(),
        transaction: ctx.accounts.transaction.key(),
        executor: ctx.accounts.owner.key(),
        timestamp: Clock::get()?.unix_timestamp
    });
    Ok(())
}

// Program errors
#[error_code]
pub enum ErrorCode {
    #[msg("The given owner is not part of this smallet.")]
    InvalidOwner,
    #[msg("Estimated execution block must satisfy delay.")]
    InvalidETA,
    #[msg("Delay greater than the maximum.")]
    DelayTooHigh,
    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,
    #[msg("Transaction is past the grace period.")]
    TransactionIsStale,
    #[msg("Transaction hasn't surpassed time lock.")]
    TransactionNotReady,
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,
    #[msg("Owner set has changed since the creation of the transaction.")]
    OwnerSetChanged,
    #[msg("Subaccount does not belong to smallet.")]
    SubaccountOwnerMismatch,
    #[msg("Buffer already finalized.")]
    BufferFinalized,
    #[msg("Buffer bundle not found.")]
    BufferBundleNotFound,
    #[msg("Buffer index specified is out of range.")]
    BufferBundleOutOfRange,
    #[msg("Buffer has not been finalized.")]
    BufferBundleNotFinalized,
    #[msg("Buffer bundle has already been executed.")]
    BufferBundleExecuted,
}
