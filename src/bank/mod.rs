//! This module contains most of the logic of the application.
//!
//! A [Bank](struct.Bank.html) is the system used to keep track of accounts and transactions, as well as apply transactions.

pub mod account;
pub mod transaction;

use account::{Account, ClientID};
use std::collections::HashMap;
use std::convert::TryFrom;
use transaction::{
    Error, Transaction, TransactionAmendment, TransactionID, TransactionInput, TransactionInputKind,
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
    pub fn perform_transaction(&mut self, ti: TransactionInput) -> Result<&Account, Error> {
        let account = self
            .accounts
            .entry(ti.client)
            .or_insert(Account::new(ti.client));

        if account.locked {
            return Err(Error::AccountFrozen);
        }

        match ti.kind {
            TransactionInputKind::Deposit => {
                account.available += ti.amount.unwrap();
                self.transactions
                    .insert(ti.tx, Transaction::try_from(ti).unwrap());
            }
            TransactionInputKind::Withdrawal => {
                let amount = ti.amount.unwrap();
                if amount > account.available {
                    return Err(Error::InsufficientFunds);
                }
                account.available -= amount;
                self.transactions
                    .insert(ti.tx, Transaction::try_from(ti).unwrap());
            }
            TransactionInputKind::Dispute => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    account.available -= prev_txn.amount;
                    account.held += prev_txn.amount;
                    prev_txn.amend(TransactionAmendment::Dispute);
                }
            }
            TransactionInputKind::Resolve => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.is_disputed() {
                        account.available += prev_txn.amount;
                        account.held -= prev_txn.amount;
                        prev_txn.amend(TransactionAmendment::Resolve);
                    }
                }
            }
            TransactionInputKind::Chargeback => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.is_disputed() {
                        account.held -= prev_txn.amount;
                        prev_txn.amend(TransactionAmendment::Chargeback);
                        account.locked = true;
                    }
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
            .perform_transaction(TransactionInput {
                client: ClientID(0),
                tx: TransactionID(0),
                amount: Some(Decimal::new(12345, 4)),
                kind: TransactionInputKind::Deposit,
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
            .perform_transaction(TransactionInput {
                client: ClientID(0),
                tx: TransactionID(0),
                amount: Some(Decimal::new(1, 4)),
                kind: TransactionInputKind::Withdrawal,
            })
            .unwrap();

        assert_eq!(Decimal::new(9, 4), account.total());
    }

    #[test]
    fn withdrawal_transaction_with_insufficient_funds() {
        let mut bank = Bank::new();
        let result = bank.perform_transaction(TransactionInput {
            client: ClientID(0),
            tx: TransactionID(0),
            amount: Some(Decimal::new(1, 4)),
            kind: TransactionInputKind::Withdrawal,
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
            .perform_transaction(TransactionInput {
                client: ClientID(0),
                tx: TransactionID(0),
                amount: None,
                kind: TransactionInputKind::Dispute,
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
            .perform_transaction(TransactionInput {
                client: ClientID(0),
                tx: TransactionID(0),
                amount: None,
                kind: TransactionInputKind::Resolve,
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
            .perform_transaction(TransactionInput {
                client: ClientID(0),
                tx: TransactionID(0),
                amount: None,
                kind: TransactionInputKind::Chargeback,
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
