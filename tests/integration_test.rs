use transactomatic::cli;

macro_rules! integration_test {
    ($(($name:ident, $in_file:expr, $out_file:expr)),*) => {
        $(
            #[test]
            fn $name() {
                let input = include_str!($in_file);
                let want = include_str!($out_file);

                let mut writer = vec![];

                cli::run(input.as_bytes(), &mut writer).unwrap();

                let got = String::from_utf8(writer).unwrap();

                // Row order isn't deterministic. This sorts all lines (including headers!) to compare.
                let mut want = want.trim().split('\n').collect::<Vec<&str>>();
                want.sort_unstable();
                let mut got = got.trim().split('\n').collect::<Vec<&str>>();
                got.sort_unstable();

                assert_eq!(want, got);
            }
        )*
    };
}

integration_test![
    // A simple transaction series with two clients and no disputes
    (
        basic_deposits_and_withdrawals,
        "simple_in1.csv",
        "simple_out1.csv"
    ),
    // A complex series with a single client but multiple disputes and an erroneous resolve
    (complex_with_disputes, "complex_in1.csv", "complex_out1.csv")
];
