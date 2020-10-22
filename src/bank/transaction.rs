use super::account::ClientID;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TransactionID(pub u32);

#[derive(Debug, PartialEq)]
pub enum Error {
    InsufficientFunds,
    AccountFrozen,
}

#[derive(Debug, PartialEq)]
pub struct TryFromError(TransactionInputKind);

#[derive(Debug)]
pub struct Transaction {
    pub client: ClientID,
    pub tx: TransactionID,
    pub kind: TransactionKind,
    pub amount: Decimal,
    amendment_history: Vec<TransactionAmendment>,
}

#[derive(Debug)]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
}

#[derive(Debug, PartialEq)]
pub enum TransactionAmendment {
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TransactionInput {
    #[serde(rename = "type")]
    pub kind: TransactionInputKind,
    pub client: ClientID,
    pub tx: TransactionID,
    pub amount: Option<Decimal>,
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InsufficientFunds => write!(f, "insufficent funds"),
            Error::AccountFrozen => write!(f, "account is frozen"),
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
        client: ClientID,
        tx: TransactionID,
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

    pub fn is_disputed(&self) -> bool {
        if let Some(TransactionAmendment::Dispute) = self.amendment_history.last() {
            return true;
        }
        false
    }

    pub fn amend(&mut self, amendment: TransactionAmendment) {
        self.amendment_history.push(amendment);
    }

    pub fn amendment_history(&self) -> &[TransactionAmendment] {
        &self.amendment_history[..]
    }
}

impl std::convert::TryFrom<TransactionInput> for Transaction {
    type Error = TryFromError;
    fn try_from(ti: TransactionInput) -> Result<Self, Self::Error> {
        match ti.kind {
            TransactionInputKind::Deposit => Ok(Transaction::new(
                ti.client,
                ti.tx,
                TransactionKind::Deposit,
                ti.amount.unwrap(),
            )),
            TransactionInputKind::Withdrawal => Ok(Transaction::new(
                ti.client,
                ti.tx,
                TransactionKind::Withdrawal,
                ti.amount.unwrap(),
            )),
            _ => Err(TryFromError(ti.kind)),
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
