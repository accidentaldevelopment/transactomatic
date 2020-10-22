use crate::bank::{transaction::TransactionInput, Bank};
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
        // Errors are to be dropped according to spec
        let _ = bank.perform_transaction(tx_input);
    }

    let mut writer = csv::Writer::from_writer(output);
    for account in bank.accounts() {
        writer.serialize(account).unwrap();
    }
    writer.flush().unwrap();
}
