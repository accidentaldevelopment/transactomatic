//! This module contains types for handling transaction instructions.

use crate::bank::{ClientID, TransactionID};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A transaction instruction from an outside source.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TransactionInstruction {
    #[serde(rename = "type")]
    pub kind: TransactionInstructionKind,
    pub client: ClientID,
    pub tx: TransactionID,
    pub amount: Option<Decimal>,
}

/// Transaction input type.  Covers all Transaction and amendment types.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionInstructionKind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
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
                        let tx: TransactionInstruction = record.unwrap();
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
            TransactionInstruction {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: Some(Decimal::from(1)),
                kind: TransactionInstructionKind::Deposit
            }
        ),
        (
            withdrawal,
            WITHDRAWAL,
            TransactionInstruction {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: Some(Decimal::from(1)),
                kind: TransactionInstructionKind::Withdrawal
            }
        ),
        (
            dispute,
            DISPUTE,
            TransactionInstruction {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: None,
                kind: TransactionInstructionKind::Dispute
            }
        ),
        (
            resolve,
            RESOLVE,
            TransactionInstruction {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: None,
                kind: TransactionInstructionKind::Resolve
            }
        ),
        (
            chargeback,
            CHARGEBACK,
            TransactionInstruction {
                client: ClientID(1),
                tx: TransactionID(1),
                amount: None,
                kind: TransactionInstructionKind::Chargeback
            }
        )
    );
}
