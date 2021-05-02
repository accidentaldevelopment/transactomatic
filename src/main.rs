#![warn(clippy::all, rust_2018_idioms, clippy::pedantic)]

use std::io;

use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, EnvFilter, Registry};
use transactomatic::cli;

const EXIT_INVALID_USAGE: i32 = 1;
const EXIT_ERROR_OPENING_FILE: i32 = 2;
const EXIT_ERROR_PROCESSING: i32 = 3;

fn main() {
    init_logging();

    let mut args = std::env::args();

    let input_file = args.nth(1).unwrap_or_else(|| {
        eprintln!("Input file must be provided");
        std::process::exit(EXIT_INVALID_USAGE);
    });

    let reader = std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .open(input_file)
        .unwrap_or_else(|e| {
            eprintln!("error opening input file: {}", e);
            std::process::exit(EXIT_ERROR_OPENING_FILE);
        });

    if let Err(err) = cli::run(reader, std::io::stdout()) {
        eprintln!("error processing transaction instructions: {:?}", err);
        std::process::exit(EXIT_ERROR_PROCESSING);
    }
}

/// Initialize logging just like `env_logger`, but default to level OFF to avoid polluting output.
fn init_logging() {
    LogTracer::init().expect("could not capture logs");
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::FULL)
        .with_writer(io::stderr);
    let subscriber = Registry::default().with(env_filter).with(layer);
    set_global_default(subscriber).expect("error creating tracing subscriber")
}
