use forth_hax::{Context, builtins::BUILT_IN_WORDS, evaluate, StepResult};

const SINGLE_LINE_CASES: &[(&str, &str)] = &[
    ("42 emit", "*"),
    (": 42 emit ;", ""),
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
