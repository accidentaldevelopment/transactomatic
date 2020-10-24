//! This module contains most of the logic of the application.
//!
//! A [Bank](struct.Bank.html) is the system used to keep track of accounts and transactions, as well as apply transactions.

pub mod account;
pub mod transaction;

use account::{Account, ClientID};
use std::collections::HashMap;
use std::convert::TryFrom;
use transaction::{
    instruction::{TransactionInstruction, TransactionInstructionKind},
    Error, Transaction, TransactionAmendment, TransactionID,
};

/// A Bank is the system used to keep track of accounts and transactions.
#[derive(Debug)]
pub struct Bank {
    accounts: HashMap<ClientID, Account>,
    transactions: HashMap<TransactionID, Transaction>,
}

impl Bank {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    /// Return an iterator over the accounts.  This a convenience so that the underlying storage doesn't have to be exposed.
    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    /// Perform a transaction based on the [TransactionInput](transaction/struct.TransactionInput.html).
    ///
    /// This method returns a Result with a reference to the affected account.
    /// This is to allow the caller to see the current state after the transaction has been applied.
    ///
    /// The Error returned does not necessarily indicate a critical error; it may just mean that the transaction wasn't applied.
    /// For example, the input could be a disputed Transaction for which the original Transaction doesn't exist.
    pub fn perform_transaction(&mut self, ti: TransactionInstruction) -> Result<&Account, Error> {
        let account = self.accounts.entry(ti.client).or_insert_with(|| {
            log::info!("creating account {:?}", ti.client);
            Account::new(ti.client)
        });

        if account.locked {
            log::warn!("account is locked {:?}", account);
            return Err(Error::AccountFrozen);
        }

        match ti.kind {
            TransactionInstructionKind::Deposit => {
                log::info!("applying transaction {:?}", ti);
                log::trace!("applying transaction {:?} to account {:?}", ti, account);
                account.available += ti.amount.unwrap();
                log::trace!("transaction applied to account {:?}", account);
                self.transactions
                    .insert(ti.tx, Transaction::try_from(ti).unwrap());
            }
            TransactionInstructionKind::Withdrawal => {
                let amount = ti.amount.unwrap();
                if amount > account.available {
                    log::error!("insufficent funds for transaction {:?}", ti);
                    return Err(Error::InsufficientFunds);
                } else {
                    log::info!("applying transaction {:?}", ti);
                    log::trace!("applying transaction {:?} to account {:?}", ti, account);
                }
                account.available -= amount;
                self.transactions
                    .insert(ti.tx, Transaction::try_from(ti).unwrap());
                log::trace!("transaction applied to account {:?}", account);
            }
            TransactionInstructionKind::Dispute => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    log::trace!("applying transaction {:?} to account {:?}", ti, account);
                    account.available -= prev_txn.amount;
                    account.held += prev_txn.amount;
                    prev_txn.amend(TransactionAmendment::Dispute);
                    log::trace!("transaction applied to account {:?}", account);
                } else {
                    log::info!("original transaction not found for instruction {:?}", ti);
                }
            }
            TransactionInstructionKind::Resolve => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.is_disputed() {
                        log::trace!("applying transaction {:?} to account {:?}", ti, account);
                        account.available += prev_txn.amount;
                        account.held -= prev_txn.amount;
                        prev_txn.amend(TransactionAmendment::Resolve);
                        log::trace!("transaction applied to account {:?}", account);
                    } else {
                        log::warn!("transaction is not in dispute: {:?}", prev_txn);
                    }
                } else {
                    log::info!("original transaction not found for instruction {:?}", ti);
                }
            }
            TransactionInstructionKind::Chargeback => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.is_disputed() {
                        log::trace!("applying transaction {:?} to account {:?}", ti, account);
                        account.held -= prev_txn.amount;
                        prev_txn.amend(TransactionAmendment::Chargeback);
                        account.locked = true;
                        log::trace!("transaction applied to account {:?}", account);
                    } else {
                        log::warn!("transaction is not in dispute: {:?}", prev_txn);
                    }
                } else {
                    log::info!("original transaction not found for instruction {:?}", ti);
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
                client: ClientID(0),
                tx: TransactionID(0),
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
            ClientID(0),
            Account {
                available: Decimal::new(10, 4),
                ..Account::new(ClientID(0))
            },
        );

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: ClientID(0),
                tx: TransactionID(0),
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
            client: ClientID(0),
            tx: TransactionID(0),
            amount: Some(Decimal::new(1, 4)),
            kind: TransactionInstructionKind::Withdrawal,
        });

        assert_eq!(result.unwrap_err(), transaction::Error::InsufficientFunds);
    }

    #[test]
    fn dispute_transaction() {
        let mut bank = Bank::new();
        bank.accounts.insert(
            ClientID(0),
            Account {
                available: Decimal::from(10),
                ..Account::new(ClientID(0))
            },
        );
        let tx = TransactionID(0);
        let txn = Transaction::new(ClientID(0), tx, TransactionKind::Deposit, Decimal::from(10));
        bank.transactions.insert(txn.tx, txn);

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: ClientID(0),
                tx: TransactionID(0),
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
            ClientID(0),
            Account {
                available: Decimal::from(5),
                held: Decimal::from(5),
                ..Account::new(ClientID(0))
            },
        );
        let tx = TransactionID(0);
        let mut txn = Transaction::new(ClientID(0), tx, TransactionKind::Deposit, Decimal::from(5));
        txn.amend(TransactionAmendment::Dispute);
        bank.transactions.insert(txn.tx, txn);

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: ClientID(0),
                tx: TransactionID(0),
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
            ClientID(0),
            Account {
                available: Decimal::from(5),
                held: Decimal::from(5),
                ..Account::new(ClientID(0))
            },
        );
        let tx = TransactionID(0);
        let mut txn = Transaction::new(ClientID(0), tx, TransactionKind::Deposit, Decimal::from(5));
        txn.amend(TransactionAmendment::Dispute);
        bank.transactions.insert(txn.tx, txn);

        let account = bank
            .perform_transaction(TransactionInstruction {
                client: ClientID(0),
                tx: TransactionID(0),
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
}
