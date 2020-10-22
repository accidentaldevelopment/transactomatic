pub mod account;
pub mod transaction;

use account::{Account, ClientID};
use std::collections::HashMap;
use transaction::{
    Error, Transaction, TransactionID, TransactionInput, TransactionInputKind, TransactionKind,
};

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
                self.transactions.insert(ti.tx, Transaction::from(ti));
            }
            TransactionInputKind::Withdrawal => {
                let amount = ti.amount.unwrap();
                if amount > account.available {
                    return Err(Error::InsufficientFunds);
                }
                account.available -= amount;
                self.transactions.insert(ti.tx, Transaction::from(ti));
            }
            TransactionInputKind::Dispute => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    match prev_txn.kind {
                        TransactionKind::Deposit(amount) | TransactionKind::Withdrawal(amount) => {
                            account.available -= amount;
                            account.held += amount;
                            prev_txn.amendment_history.push(TransactionKind::Dispute);
                        }
                        _ => {}
                    }
                }
            }
            TransactionInputKind::Resolve => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.is_disputed() {
                        match prev_txn.kind {
                            TransactionKind::Deposit(amount)
                            | TransactionKind::Withdrawal(amount) => {
                                account.available += amount;
                                account.held -= amount;
                                prev_txn.amendment_history.push(TransactionKind::Resolve);
                            }
                            _ => {}
                        }
                    }
                }
            }
            TransactionInputKind::Chargeback => {
                if let Some(prev_txn) = self.transactions.get_mut(&ti.tx) {
                    if prev_txn.is_disputed() {
                        match prev_txn.kind {
                            TransactionKind::Deposit(amount)
                            | TransactionKind::Withdrawal(amount) => {
                                account.held -= amount;
                                prev_txn.amendment_history.push(TransactionKind::Chargeback);
                            }
                            _ => {}
                        }
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
        bank.transactions.insert(
            TransactionID(0),
            Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Deposit(Decimal::from(10)),
                amendment_history: vec![],
            },
        );

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
        bank.transactions.insert(
            TransactionID(0),
            Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Deposit(Decimal::from(5)),
                amendment_history: vec![TransactionKind::Dispute],
            },
        );

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
        bank.transactions.insert(
            TransactionID(0),
            Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Deposit(Decimal::from(5)),
                amendment_history: vec![TransactionKind::Dispute],
            },
        );

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
    }
}
