//! This module contains most of the logic of the application.
//!
//! A [Bank](struct.Bank.html) is the system used to keep track of accounts and transactions, as well as apply transactions.

use account::{Account, AccountId};
use std::collections::HashMap;
use std::convert::TryFrom;
use tracing::instrument;
use transaction::{
    instruction::{TransactionInstruction, TransactionInstructionKind},
    Error, Transaction, TransactionAmendment, TransactionId,
};

pub mod account;
pub mod transaction;

/// A Bank is the system used to keep track of accounts and transactions.
#[derive(Debug, Default)]
pub struct Bank {
    accounts: HashMap<AccountId, Account>,
    transactions: HashMap<TransactionId, Transaction>,
}

impl Bank {
    #[must_use]
    pub fn new() -> Self {
        Bank::default()
    }

    /// Return an iterator over the accounts.  This a convenience so that the underlying storage doesn't have to be exposed.
    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    /// Perform a transaction based on the [`TransactionInput`](transaction/struct.TransactionInput.html).
    ///
    /// This method returns a Result with a reference to the affected account.
    /// This is to allow the caller to see the current state after the transaction has been applied.
    ///
    /// The Error returned does not necessarily indicate a critical error; it may just mean that the transaction wasn't applied.
    /// For example, the input could be a disputed Transaction for which the original Transaction doesn't exist.
    ///
    /// # Panics
    ///
    /// Panics if there is an error converting the `TransactionInstruction` into
    /// a `Transaction`. Both types are controlled in this codebase so this
    /// should never happen.
    ///
    /// # Errors
    ///
    /// Will return `Err` if it can't process the instruction.
    #[instrument(skip(self))]
    pub fn perform_transaction(&mut self, ti: TransactionInstruction) -> Result<&Account, Error> {
        let account = self.accounts.entry(ti.client).or_insert_with(|| {
            tracing::info!("creating account");
            Account::new(ti.client)
        });

        if account.locked {
            tracing::warn!(?account, "account is locked");
            return Err(Error::AccountFrozen);
        }

        if let Some(amount) = &ti.amount {
            if amount.is_sign_negative() {
                return Err(Error::NegativeAmount);
            }
        }

        match ti.kind {
            TransactionInstructionKind::Deposit => match self.transactions.entry(ti.tx) {
                std::collections::hash_map::Entry::Occupied(_) => {
                    tracing::error!(id = ?ti.tx, "transaction id already exists")
                }
                std::collections::hash_map::Entry::Vacant(_) => {
                    tracing::info!("applying transaction");
                    tracing::trace!(?account, "applying transaction");
                    account.available += ti.amount.unwrap();
                    tracing::trace!(?account, "transaction applied to account");
                    self.transactions
                        .insert(ti.tx, Transaction::try_from(ti).unwrap());
                }
            },
            TransactionInstructionKind::Withdrawal => match self.transactions.entry(ti.tx) {
                std::collections::hash_map::Entry::Occupied(_) => {
                    tracing::error!(id = ?ti.tx, "transaction id already exists")
                }
                std::collections::hash_map::Entry::Vacant(_) => {
                    let amount = ti.amount.unwrap();
                    if amount > account.available {
                        tracing::error!("insufficient funds for transaction");
                        return Err(Error::InsufficientFunds);
                    }

                    tracing::info!("applying transaction");
                    tracing::trace!(?account, "applying transaction",);
                    account.available -= amount;
                    self.transactions
                        .insert(ti.tx, Transaction::try_from(ti).unwrap());
                    tracing::trace!(?account, "transaction applied to account");
                }
            },
            TransactionInstructionKind::Dispute => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.client == ti.client {
                        tracing::trace!(?account, "applying transaction to account");
                        account.available -= prev_txn.amount;
                        account.held += prev_txn.amount;
                        prev_txn.amend(TransactionAmendment::Dispute);
                        tracing::trace!(?account, "transaction applied to account");
                    } else {
                        tracing::error!("transaction client doesn't match instruction client");
                    }
                } else {
                    tracing::info!("original transaction not found for instruction");
                }
            }
            TransactionInstructionKind::Resolve => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.client == ti.client {
                        if prev_txn.is_disputed() {
                            tracing::trace!(?account, "applying transaction to account");
                            account.available += prev_txn.amount;
                            account.held -= prev_txn.amount;
                            prev_txn.amend(TransactionAmendment::Resolve);
                            tracing::trace!(?account, "transaction applied to account");
                        } else {
                            tracing::warn!(txn = ?prev_txn, "transaction is not in dispute");
                        }
                    } else {
                        tracing::error!(
                            prev_tx_client = ?prev_txn.client,
                            instruction_client = ?ti.client,
                            "transaction client doesn't match instruction client"
                        );
                    }
                } else {
                    tracing::info!("original transaction not found for instruction");
                }
            }
            TransactionInstructionKind::Chargeback => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.is_disputed() {
                        tracing::trace!(?account, "applying transaction to account");
                        account.held -= prev_txn.amount;
                        prev_txn.amend(TransactionAmendment::Chargeback);
                        account.locked = true;
                        tracing::trace!(?account, "transaction applied to account");
                    } else {
                        tracing::warn!(txn = ?prev_txn, "transaction is not in dispute");
                    }
                } else {
                    tracing::info!("original transaction not found for instruction");
                }
            }
        }
        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use super::transaction::TransactionKind;
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn deposit_transaction() {
        let mut bank = Bank::new();
        let account = bank
            .perform_transaction(TransactionInstruction {
                client: AccountId(0),
                tx: TransactionId(0),
                amount: Some(Decimal::new(12345, 4)),
                kind: TransactionInstructionKind::Deposit,
            })
            .unwrap();

        assert_eq!(Decimal::new(12345, 4), account.total());
    }

    #[test]
    fn withdrawal_transaction() {
        let mut bank = Bank::new();
        bank.accounts.insert(
            AccountId(0),
            Account {
                available: Decimal::new(10, 4),
                ..Account::new(AccountId(0))
            },
        );

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: AccountId(0),
                tx: TransactionId(0),
                amount: Some(Decimal::new(1, 4)),
                kind: TransactionInstructionKind::Withdrawal,
            })
            .unwrap();

        assert_eq!(Decimal::new(9, 4), account.total());
    }

    #[test]
    fn withdrawal_transaction_with_insufficient_funds() {
        let mut bank = Bank::new();
        let result = bank.perform_transaction(TransactionInstruction {
            client: AccountId(0),
            tx: TransactionId(0),
            amount: Some(Decimal::new(1, 4)),
            kind: TransactionInstructionKind::Withdrawal,
        });

        assert_eq!(result.unwrap_err(), transaction::Error::InsufficientFunds);
    }

    #[test]
    fn dispute_transaction() {
        let mut bank = Bank::new();
        bank.accounts.insert(
            AccountId(0),
            Account {
                available: Decimal::from(10),
                ..Account::new(AccountId(0))
            },
        );
        let tx = TransactionId(0);
        let txn = Transaction::new(
            AccountId(0),
            tx,
            TransactionKind::Deposit,
            Decimal::from(10),
        );
        bank.transactions.insert(txn.tx, txn);

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: AccountId(0),
                tx: TransactionId(0),
                amount: None,
                kind: TransactionInstructionKind::Dispute,
            })
            .unwrap();

        assert_eq!(account.available, Decimal::from(0));
        assert_eq!(account.total(), Decimal::from(10));
        assert_eq!(account.held, Decimal::from(10));
        assert_eq!(
            bank.transactions[&tx].amendment_history(),
            [TransactionAmendment::Dispute]
        );
    }

    #[test]
    fn resolve_transaction() {
        let mut bank = Bank::new();
        bank.accounts.insert(
            AccountId(0),
            Account {
                available: Decimal::from(5),
                held: Decimal::from(5),
                ..Account::new(AccountId(0))
            },
        );
        let tx = TransactionId(0);
        let mut txn =
            Transaction::new(AccountId(0), tx, TransactionKind::Deposit, Decimal::from(5));
        txn.amend(TransactionAmendment::Dispute);
        bank.transactions.insert(txn.tx, txn);

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: AccountId(0),
                tx: TransactionId(0),
                amount: None,
                kind: TransactionInstructionKind::Resolve,
            })
            .unwrap();

        assert_eq!(account.available, Decimal::from(10));
        assert_eq!(account.total(), Decimal::from(10));
        assert_eq!(account.held, Decimal::from(0));
        assert_eq!(
            bank.transactions[&tx].amendment_history(),
            [TransactionAmendment::Dispute, TransactionAmendment::Resolve]
        );
    }

    #[test]
    fn chargeback_transaction() {
        let mut bank = Bank::new();
        bank.accounts.insert(
            AccountId(0),
            Account {
                available: Decimal::from(5),
                held: Decimal::from(5),
                ..Account::new(AccountId(0))
            },
        );
        let tx = TransactionId(0);
        let mut txn =
            Transaction::new(AccountId(0), tx, TransactionKind::Deposit, Decimal::from(5));
        txn.amend(TransactionAmendment::Dispute);
        bank.transactions.insert(txn.tx, txn);

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: AccountId(0),
                tx: TransactionId(0),
                amount: None,
                kind: TransactionInstructionKind::Chargeback,
            })
            .unwrap();

        assert_eq!(account.available, Decimal::from(5));
        assert_eq!(account.total(), Decimal::from(5));
        assert_eq!(account.held, Decimal::from(0));
        assert_eq!(account.locked, true);
        assert_eq!(
            bank.transactions[&tx].amendment_history(),
            [
                TransactionAmendment::Dispute,
                TransactionAmendment::Chargeback
            ]
        );
    }

    #[test]
    fn negative_amount() {
        let mut bank = Bank::new();
        let result = bank.perform_transaction(TransactionInstruction {
            client: AccountId(0),
            tx: TransactionId(0),
            amount: Some(Decimal::new(-1, 4)),
            kind: TransactionInstructionKind::Deposit,
        });

        assert!(matches!(result, Err(Error::NegativeAmount)));
    }
}
