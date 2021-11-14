use anachro_forth_host::{builtins::BUILT_IN_WORDS, evaluate, Context, StepResult};

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
    (
        "0 42 emit if 42 emit else 42 emit 42 emit then 42 emit",
        "****",
    ),
    (
        "1 42 emit if 42 emit else 42 emit 42 emit then 42 emit",
        "***",
    ),
    // Comparison operators
    ("0 1 < if 42 emit then", "*"),
    ("1 0 < if 42 emit then", ""),
    ("0 1 > if 42 emit then", ""),
    ("1 0 > if 42 emit then", "*"),
    ("1 0 = if 42 emit then", ""),
    ("0 1 = if 42 emit then", ""),
    ("1 1 = if 42 emit then", "*"),
    ("0 0 = if 42 emit then", "*"),

    // Nested loops - doesn't work!
    // ("0 0 if 42 emit if 42 emit else 42 emit 42 emit then then", ""),
    // ("1 0 if 42 emit if 42 emit else 42 emit 42 emit then then", "***"),
    // ("1 1 if 42 emit if 42 emit else 42 emit 42 emit then then", "**"),
];

const MULTI_LINE_CASES: &[(&str, &str)] = &[
    (
        r#"
            : star 42 emit ;
            star
        "#,
        "*",
    ),
    (
        r#"
            : test 10 0 do 42 emit LOOP ;
            test
        "#,
        "**********",
    ),
    (
        r#"
            : star 42 emit ;
            : test star 10 0 do star LOOP star ;
            test
        "#,
        "************",
    ),

    // Nested loops: Not working!
    // (
    //     r#"
    //         : star 42 emit ;
    //         : test 10 0 do 10 0 do star loop loop ;
    //         test
    //     "#,
    //     "************",
    // ),
];

/// Creates a clean engine
#[test]
fn single_lines() {
    for (cases, output) in SINGLE_LINE_CASES {
        let mut ctxt = Context::with_builtins(BUILT_IN_WORDS);
        println!("{:?} => {:?}", cases, output);
        evaluate(&mut ctxt, s(cases)).unwrap();
        while let StepResult::Working = ctxt.step().unwrap() {
            // ...
        }
        assert_eq!(output, &ctxt.output(),);

        assert_eq!(0, ctxt.data_stack().len());
        assert_eq!(0, ctxt.return_stack().len());
        assert_eq!(0, ctxt.flow_stack().len());
    }
}

#[test]
fn multi_lines() {
    for (cases, output) in MULTI_LINE_CASES {
        let mut ctxt = Context::with_builtins(BUILT_IN_WORDS);

        for cline in cases.lines().map(str::trim) {
            println!("{:?}", cline);
            evaluate(&mut ctxt, s(cline)).unwrap();
            while let StepResult::Working = ctxt.step().unwrap() {
                // ...
            }
        }

        assert_eq!(output, &ctxt.output(),);

        assert_eq!(0, ctxt.data_stack().len());
        assert_eq!(0, ctxt.return_stack().len());
        assert_eq!(0, ctxt.flow_stack().len());
    }
}

fn s(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_string).collect()
}
