use crate::bank::{transaction::instruction::TransactionInstruction, Bank};
use std::io;

pub fn run<R: io::Read, W: io::Write>(
    input: R,
    output: W,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .comment(Some(b'#'))
        .from_reader(input);

    let mut bank = Bank::new();

    for ti in reader.deserialize() {
        let tx_input: TransactionInstruction = ti.unwrap();
        log::debug!("transaction instruction {:?}", tx_input);
        // Errors are to be dropped according to spec
        if let Err(e) = bank.perform_transaction(tx_input) {
            log::error!("error applying transaction: {:?}", e);
        }
    }

    let mut writer = csv::Writer::from_writer(output);
    for account in bank.accounts() {
        writer.serialize(account)?;
    }
    writer.flush()?;
    Ok(())
}
