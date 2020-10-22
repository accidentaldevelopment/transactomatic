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

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    pub client: ClientID,
    pub tx: TransactionID,
    pub is_disputed: bool,
    #[serde(flatten)]
    pub kind: TransactionKind,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum TransactionKind {
    Deposit(Decimal),
    Withdrawal(Decimal),
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
