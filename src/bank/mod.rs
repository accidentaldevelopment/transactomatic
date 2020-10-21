#[warn(clippy::all)]
mod account;
mod transaction;

use account::{Account, ClientID};
use std::collections::HashMap;
use transaction::{Error, Transaction, TransactionID, TransactionKind};

pub struct Bank {
    accounts: HashMap<ClientID, Account>,
    transactions: HashMap<TransactionID, Transaction>,

    decimal_precision: u32,
}

impl Bank {
    pub fn new() -> Self {
        Self::with_decimal_precision(4)
    }

    pub fn with_decimal_precision(decimal_precision: u32) -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
            decimal_precision,
        }
    }

    pub fn perform_transaction(&mut self, transaction: Transaction) -> Result<&Account, Error> {
        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert(Account::new(transaction.client));

        if account.locked {
            return Err(Error::AccountFrozen);
        }

        match transaction.kind {
            TransactionKind::Deposit(amount) => account.available += amount,
            TransactionKind::Withdrawal(amount) => {
                if amount > account.available {
                    return Err(Error::InsufficientFunds);
                }
                account.available -= amount
            }
            TransactionKind::Dispute => {
                if let Some(prev_txn) = self.transactions.get(&transaction.tx) {
                    match prev_txn.kind {
                        TransactionKind::Deposit(amount) | TransactionKind::Withdrawal(amount) => {
                            account.available -= amount;
                            account.held += amount;
                        }
                        _ => {}
                    }
                }
            }
            TransactionKind::Resolve => {
                if let Some(prev_txn) = self.transactions.get(&transaction.tx) {
                    match prev_txn.kind {
                        TransactionKind::Deposit(amount) | TransactionKind::Withdrawal(amount) => {
                            account.available += amount;
                            account.held -= amount;
                        }
                        _ => {}
                    }
                }
            }
            TransactionKind::Chargeback => {
                if let Some(prev_txn) = self.transactions.get(&transaction.tx) {
                    match prev_txn.kind {
                        TransactionKind::Deposit(amount) | TransactionKind::Withdrawal(amount) => {
                            account.held -= amount;
                        }
                        _ => {}
                    }
                    account.locked = true;
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
            .perform_transaction(Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Deposit(Decimal::new(12345, 4)),
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
            .perform_transaction(Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Withdrawal(Decimal::new(1, 4)),
            })
            .unwrap();

        assert_eq!(Decimal::new(9, 4), account.total());
    }

    #[test]
    fn withdrawal_transaction_with_insufficient_funds() {
        let mut bank = Bank::new();
        let result = bank.perform_transaction(Transaction {
            client: ClientID(0),
            tx: TransactionID(0),
            kind: TransactionKind::Withdrawal(Decimal::new(1, 4)),
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
            },
        );

        let account = bank
            .perform_transaction(Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Dispute,
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
            },
        );

        let account = bank
            .perform_transaction(Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Resolve,
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
            },
        );

        let account = bank
            .perform_transaction(Transaction {
                client: ClientID(0),
                tx: TransactionID(0),
                kind: TransactionKind::Chargeback,
            })
            .unwrap();

        assert_eq!(account.available, Decimal::from(5));
        assert_eq!(account.total(), Decimal::from(5));
        assert_eq!(account.held, Decimal::from(0));
        assert_eq!(account.locked, true);
    }
}
