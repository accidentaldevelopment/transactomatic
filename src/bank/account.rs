use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        self.available + self.held
    }
}
