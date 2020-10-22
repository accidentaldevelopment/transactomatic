use transactomatic::cli;

const EXIT_INVALID_USAGE: i32 = 1;
const EXIT_ERROR_OPENING_FILE: i32 = 2;

fn main() {
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

    cli::run(reader, std::io::stdout());
}
