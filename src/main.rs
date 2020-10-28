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

/// Initialize logging just like env_logger, but default to level OFF to avoid polluting output.
fn init_logging() {
    if let Err(_) = std::env::var("RUST_LOG") {
        pretty_env_logger::formatted_builder()
            .filter_level(log::LevelFilter::Off)
            .init();
    } else {
        pretty_env_logger::init();
    }
}
