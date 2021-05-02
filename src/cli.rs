use crate::bank::{transaction::instruction::TransactionInstruction, Bank};
use std::io;

/// # Errors
///
/// Will return an `Err` if there is a problem running the main application logic.
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
        let tx_input: TransactionInstruction = match ti {
            Ok(ti) => ti,
            Err(err) => {
                tracing::error!(?err, "error deserializing transaction instruction");
                continue;
            }
        };
        tracing::debug!("transaction instruction {:?}", tx_input);
        // Errors are to be dropped according to spec
        if let Err(err) = bank.perform_transaction(tx_input) {
            tracing::error!(?err, "error applying transaction");
        }
    }

    let mut writer = csv::Writer::from_writer(output);
    for account in bank.accounts() {
        writer.serialize(account)?;
    }
    Ok(())
}
