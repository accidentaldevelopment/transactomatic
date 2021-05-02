use rust_decimal::Decimal;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct AccountId(pub u16);

#[derive(Debug)]
pub struct Account {
    pub client: AccountId,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    #[must_use]
    pub fn new(client: AccountId) -> Self {
        Self {
            client,
            available: Decimal::from(0),
            held: Decimal::from(0),
            locked: false,
        }
    }

    /// Total balance isn't stored internally to avoid having to remember updating it every time.
    #[must_use]
    pub fn total(&self) -> Decimal {
        let mut total = self.available + self.held;
        total.rescale(4);
        total
    }
}

// Custom serializer implementation so that the total is included in the output.
impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut available = self.available;
        available.rescale(4);
        let mut held = self.held;
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
