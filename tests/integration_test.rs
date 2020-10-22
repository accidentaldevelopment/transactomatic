use std::path::PathBuf;
use transactomatic::cli;

macro_rules! integration_test {
    ($(($name:ident, $in_file:expr, $out_file:expr)),*) => {
        $(
            #[test]
            fn $name() {
                let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                base.push("tests");
                let mut input_path = base.clone();
                input_path.push($in_file);
                let mut output_path = base.clone();
                output_path.push($out_file);

                let input = std::fs::read_to_string(input_path).unwrap();
                let want = std::fs::read_to_string(output_path).unwrap();

                let mut writer = vec![];

                cli::run(input.as_bytes(), &mut writer);

                let got = String::from_utf8(writer).unwrap();

                // Row order isn't deterministic. This sorts all lines (including headers!) to compare.
                let mut want = want.trim().split('\n').collect::<Vec<&str>>();
                want.sort();
                let mut got = got.trim().split('\n').collect::<Vec<&str>>();
                got.sort();

                assert_eq!(want, got);
            }
        )*
    };
}

integration_test![
    (
        basic_deposits_and_withdrawals,
        "simple_in1.csv",
        "simple_out1.csv"
    ),
    (complex_with_disputes, "complex_in1.csv", "complex_out1.csv")
];
