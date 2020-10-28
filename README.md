# Transactomatic

A simple transaction engine.

## Running

This application reads in a list of transaction instructions from a file specified on the command line. It will exit in error if a file is not supplied.

    cargo run -- input_file.csv

## Logging

Transactomatic uses [pretty_env_logging](https://docs.rs/pretty_env_logger/0.4.0/pretty_env_logger). Logging configuration is performed by that library. The default level is overridden to be `OFF` instead of `ERROR`; this prevents log output from polluting the rest of the output.
Logs are written to `STDERR` so anything parsing `STDOUT` should be unaffected, but just in case the default level is still `OFF`.

## Libraries

- csv – For parsing and writing CSV data.
- serde – For (de)serialization.
- rust_decimal – For high precision floating point calculations.

## Assumptions

- Input is valid. No invalid CSV, strings instead of numbers, etc.
- Provided documentation specifies exactly what operations to perform for various disputes and resolutions. This application performs exactly what is laid out in that documentation, regardless of whether the original transaction was a deposit or a withdrawal.

## Testing

Unit and integration tests have been written based on my understanding of the problem. Unit tests are in `tests` modules and integration tests are in [tests](tests).

The integration tests run the application and compare output. Output row order is not deterministic so the tests will split actual and expected output into lists, sort them, and then compare.

Tests can be run with the standard `cargo test` command and options.

## ToDos

- The transaction model became a little overcomplicated; it could probably be simplified.
- More tests! (And then some more tests)
- ~~Add logging~~
