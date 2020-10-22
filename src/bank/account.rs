use rust_decimal::Decimal;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ClientID(pub u16);

#[derive(Debug)]
pub struct Account {
    pub client: ClientID,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn new(client: ClientID) -> Self {
        Self {
            client,
            available: Decimal::from(0),
            held: Decimal::from(0),
            locked: false,
        }
    }

    pub fn total(&self) -> Decimal {
        let mut total = self.available + self.held;
        total.rescale(4);
        total
    }
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut available = Decimal::from(self.available);
        available.rescale(4);
        let mut held = Decimal::from(self.held);
        held.rescale(4);

        let mut s = serializer.serialize_struct("Account", 5)?;
        s.serialize_field("client", &self.client)?;
        s.serialize_field("available", &available)?;
        s.serialize_field("held", &held)?;
        s.serialize_field("total", &self.total())?;
        s.serialize_field("locked", &self.locked)?;
        s.end()
    }
}
