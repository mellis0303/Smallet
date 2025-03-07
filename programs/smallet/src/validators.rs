//! Account validators.
use crate::*;

impl<'info> Validate<'info> for CreateSmallet<'info> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl<'info> Validate<'info> for Auth<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(
            self.smallet.to_account_info().is_signer,
            "smallet.is_signer"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for CreateTransaction<'info> {
    fn validate(&self) -> Result<()> {
    	// owner_index check happens later
        Ok(())
    }
}

impl<'info> Validate<'info> for ExecuteTransaction<'info> {
    fn validate(&self) -> Result<()> {
        assert_keys_eq!(
            self.smallet,
            self.transaction.smallet,
            "smallet"
        );
        invariant!(
            self.smallet.owner_set_seqno == self.transaction.owner_set_seqno,
            OwnerSetChanged
        );
		// Checking to see if this has been executed already
        invariant!(self.transaction.executed_at == -1, AlreadyExecuted);

        let eta = self.transaction.eta;
        let clock = Clock::get()?;
        let current_ts = clock.unix_timestamp;
        msg!("current_ts: {}; eta: {}", current_ts, eta);
        // Has transaction surpassed timelock?
        invariant!(current_ts >= eta, TransactionNotReady);
        if eta != NO_ETA {
        	// Has grace period passed?
            invariant!(
                current_ts <= unwrap_int!(eta.checked_add(self.smallet.grace_period)),
                TransactionIsStale
            );
        }
		// Do we have enough signers to execute the TX?
        let sig_count = self.transaction.num_signers();
        invariant!(
            (sig_count as u64) >= self.smallet.threshold,
            NotEnoughSigners
        );
		// ensure that the owner is a signer
        // this prevents common frontrunning/flash loan attacks
        self.smallet.try_owner_index(self.owner.key())?;

        Ok(())
    }
}

impl<'info> Validate<'info> for OwnerInvokeInstruction<'info> {
    fn validate(&self) -> Result<()> {
        self.smallet.try_owner_index(self.owner.key())?;
        Ok(())
    }
}

impl<'info> Validate<'info> for CreateSubaccountInfo<'info> {
    fn validate(&self) -> Result<()> {
    	// no validation necessary
        Ok(())
    }
}
