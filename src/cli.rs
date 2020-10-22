use crate::bank::{
    account::ClientID,
    transaction::{Transaction, TransactionID, TransactionKind},
    Bank,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io;

pub fn run<R: io::Read, W: io::Write>(input: R, output: W) {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .comment(Some(b'#'))
        .from_reader(input);

    let mut bank = Bank::new();

    for ti in reader.deserialize() {
        let tx_input: TransactionInput = ti.unwrap();
        let txn = Transaction::from(tx_input);
        // Errors are to be dropped according to spec
        let _ = bank.perform_transaction(txn);
    }

    let mut writer = csv::Writer::from_writer(output);
    for account in bank.accounts() {
        writer.serialize(account).unwrap();
    }
    writer.flush().unwrap();
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TransactionInput {
    #[serde(rename = "type")]
    kind: TransactionInputKind,
    client: ClientID,
    tx: TransactionID,
    amount: Option<Decimal>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionInputKind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl std::convert::From<TransactionInput> for Transaction {
    fn from(ti: TransactionInput) -> Self {
        match ti.kind {
            TransactionInputKind::Deposit => Transaction {
                client: ti.client,
                tx: ti.tx,
                is_disputed: false,
                kind: TransactionKind::Deposit(ti.amount.unwrap()),
            },
            TransactionInputKind::Withdrawal => Transaction {
                client: ti.client,
                tx: ti.tx,
                is_disputed: false,
                kind: TransactionKind::Withdrawal(ti.amount.unwrap()),
            },
            TransactionInputKind::Dispute => Transaction {
                client: ti.client,
                tx: ti.tx,
                is_disputed: false,
                kind: TransactionKind::Dispute,
            },
            TransactionInputKind::Resolve => Transaction {
                client: ti.client,
                tx: ti.tx,
                is_disputed: true,
                kind: TransactionKind::Resolve,
            },
            TransactionInputKind::Chargeback => Transaction {
                client: ti.client,
                tx: ti.tx,
                is_disputed: true,
                kind: TransactionKind::Chargeback,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEPOSIT: &'static str = r#"type, client, tx, amount
deposit, 1, 1, 1.0
"#;

    const WITHDRAWAL: &'static str = r#"type, client, tx, amount
withdrawal, 1, 1, 1.0
"#;

    const DISPUTE: &'static str = r#"type, client, tx, amount

dispute, 1, 1,
"#;

    const RESOLVE: &'static str = r#"type, client, tx, amount
resolve, 1, 1,
"#;

    const CHARGEBACK: &'static str = r#"type, client, tx, amount
chargeback, 1, 1
"#;

    macro_rules! test_parse {
        ($(($name:tt, $input:expr, $output:expr)),*) => {
            $(
                #[test]
                fn $name() {
                    let mut r = csv::ReaderBuilder::new()
                        .trim(csv::Trim::All)
                        .flexible(true)
                        .from_reader($input.as_bytes());
                    for record in r.deserialize() {
                        let tx: TransactionInput = record.unwrap();
                        assert_eq!($output, tx);
                    }
                }
            )*
        };
    }

    test_parse!(
        (
            deposit,
            DEPOSIT,
            TransactionInput {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: Some(Decimal::from(1)),
                kind: TransactionInputKind::Deposit
            }
        ),
        (
            withdrawal,
            WITHDRAWAL,
            TransactionInput {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: Some(Decimal::from(1)),
                kind: TransactionInputKind::Withdrawal
            }
        ),
        (
            dispute,
            DISPUTE,
            TransactionInput {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: None,
                kind: TransactionInputKind::Dispute
            }
        ),
        (
            resolve,
            RESOLVE,
            TransactionInput {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: None,
                kind: TransactionInputKind::Resolve
            }
        ),
        (
            chargeback,
            CHARGEBACK,
            TransactionInput {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: None,
                kind: TransactionInputKind::Chargeback
            }
        )
    );
}
