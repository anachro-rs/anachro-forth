use forth_hax::{Context, builtins::BUILT_IN_WORDS, evaluate, StepResult};

const SINGLE_LINE_CASES: &[(&str, &str)] = &[
    // Basic output
    ("42 emit", "*"),

    // Basic compilation
    (": 42 emit ;", ""),

    // Basic if
    ("0 if 42 emit then", ""),
    ("1 if 42 emit then", "*"),
    ("0 42 emit if 42 emit then 42 emit", "**"),
    ("1 42 emit if 42 emit then 42 emit", "***"),

    // Basic if/else
    ("0 if 42 emit else 42 emit 42 emit then", "**"),
    ("1 if 42 emit else 42 emit 42 emit then", "*"),
    ("0 42 emit if 42 emit else 42 emit 42 emit then 42 emit", "****"),
    ("1 42 emit if 42 emit else 42 emit 42 emit then 42 emit", "***"),

    // Comparison operators
    ("0 1 < if 42 emit then", "*"),
    ("1 0 < if 42 emit then", ""),
    ("0 1 > if 42 emit then", ""),
    ("1 0 > if 42 emit then", "*"),
    ("1 0 = if 42 emit then", ""),
    ("0 1 = if 42 emit then", ""),
    ("1 1 = if 42 emit then", "*"),
    ("0 0 = if 42 emit then", "*"),

];

/// Creates a clean engine
#[test]
fn single_lines() {
    for (cases, output) in SINGLE_LINE_CASES {
        let mut ctxt = Context::with_builtins(BUILT_IN_WORDS);
            println!("{:?} => {:?}", cases, output);
            evaluate(&mut ctxt, s(cases)).unwrap();
            while let StepResult::Working = ctxt.step() {
                // ...
            }
            assert_eq!(
                output,
                &ctxt.output(),
            );

            assert_eq!(0, ctxt.data_stack().len());
            assert_eq!(0, ctxt.return_stack().len());
            assert_eq!(0, ctxt.flow_stack().len());
    }


}

fn s(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_string).collect()
}
