//! This module contains types for dealing with realized transactions.
//!
//! Three scenarios are covered by this module:
//! 1. A new transaction input has come in.
//! 2. A new transaction is created.
//! 3. An existing transaction is adjusted.
//!
//! It's important to note with Number 3 that the original transaction keeps its original data and amendment are added to history.
//! Once a transaction has been created its initial data is not modified.

pub mod instruction;

use super::account::AccountId;
use instruction::{TransactionInstruction, TransactionInstructionKind};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TransactionId(pub u32);

/// Errors related to performing transactions
#[derive(Debug, PartialEq)]
pub enum Error {
    InsufficientFunds,
    AccountFrozen,
    NegativeAmount,
}

/// Errors related to creating a transaction from an input.
#[derive(Debug, PartialEq)]
pub struct TryFromError(TransactionInstructionKind);

/// A realized transaction.
#[derive(Debug)]
pub struct Transaction {
    pub client: AccountId,
    pub tx: TransactionId,
    pub kind: TransactionKind,
    pub amount: Decimal,
    amendment_history: Vec<TransactionAmendment>,
}

/// Type of original transaction
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
}

/// An amendment/adjustment to an existing Transaction.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, PartialEq)]
pub enum TransactionAmendment {
    Dispute,
    Resolve,
    Chargeback,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InsufficientFunds => write!(f, "insufficient funds"),
            Error::AccountFrozen => write!(f, "account is frozen"),
            Error::NegativeAmount => write!(f, "amount is negative"),
        }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for TryFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "can't create transaction from input kind {:?}", self.0)
    }
}

impl std::error::Error for TryFromError {}

impl Transaction {
    pub fn new<D: Into<Decimal>>(
        client: AccountId,
        tx: TransactionId,
        kind: TransactionKind,
        amount: D,
    ) -> Self {
        Self {
            client,
            tx,
            kind,
            amount: amount.into(),
            amendment_history: vec![],
        }
    }

    /// Returns `true` if the transaction is in dispute.  That is, its last amendment is Dispute.
    #[must_use]
    pub fn is_disputed(&self) -> bool {
        if let Some(TransactionAmendment::Dispute) = self.amendment_history.last() {
            return true;
        }
        false
    }

    pub fn amend(&mut self, amendment: TransactionAmendment) {
        self.amendment_history.push(amendment);
    }

    #[must_use]
    /// Returns a read-only view into the transaction's history.
    pub fn amendment_history(&self) -> &[TransactionAmendment] {
        &self.amendment_history[..]
    }
}

impl std::convert::TryFrom<TransactionInstruction> for Transaction {
    type Error = TryFromError;

    /// Attempt to build a transaction from the input.  This only works if the
    /// input type is a [`TransactionKind`](TransactionKind) and not a
    /// [`TransactionAmendment`](TransactionAmendment).
    fn try_from(ti: TransactionInstruction) -> Result<Self, Self::Error> {
        match ti.kind {
            TransactionInstructionKind::Deposit => Ok(Transaction::new(
                ti.client,
                ti.tx,
                TransactionKind::Deposit,
                ti.amount.unwrap(),
            )),
            TransactionInstructionKind::Withdrawal => Ok(Transaction::new(
                ti.client,
                ti.tx,
                TransactionKind::Withdrawal,
                ti.amount.unwrap(),
            )),
            _ => Err(TryFromError(ti.kind)),
        }
    }
}
