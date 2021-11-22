use anachro_forth_core::{std_rt::std_builtins, StepResult, WhichToken};
use anachro_forth_host::{evaluate, Context};

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
    //         : test 3 0 do 4 0 do star loop loop ;
    //         test
    //     "#,
    //     "**************",
    // ),
];

/// Creates a clean engine
#[test]
fn single_lines() {
    for (cases, output) in SINGLE_LINE_CASES {
        let mut ctxt = Context::with_builtins(std_builtins());
        println!("{:?} => {:?}", cases, output);
        evaluate(&mut ctxt, s(cases)).unwrap();
        loop {
            match ctxt.step().unwrap() {
                StepResult::Done => break,
                StepResult::Working(WhichToken::Single(ft)) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time
                    ft.exec(&mut ctxt.rt).unwrap();
                }
                StepResult::Working(WhichToken::Ref(rtw)) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time

                    let c = ctxt
                        .dict
                        .data
                        .get(&rtw.tok)
                        .and_then(|n| n.inner.get(rtw.idx))
                        .map(|n| n.clone().word);

                    ctxt.rt.provide_seq_tok(c).unwrap();
                }
            }
        }
        assert_eq!(output, &ctxt.output());

        assert_eq!(0, ctxt.rt.data_stk.data().len());
        assert_eq!(0, ctxt.rt.ret_stk.data().len());
        assert_eq!(0, ctxt.rt.flow_stk.data().len());
    }
}

#[test]
fn multi_lines() {
    for (cases, output) in MULTI_LINE_CASES {
        let mut ctxt = Context::with_builtins(std_builtins());

        for cline in cases.lines().map(str::trim) {
            println!("{:?}", cline);
            evaluate(&mut ctxt, s(cline)).unwrap();
            loop {
                match ctxt.step().unwrap() {
                    StepResult::Done => break,
                    StepResult::Working(WhichToken::Single(ft)) => {
                        // The runtime yields back at every call to a "builtin". Here, I
                        // call the builtin immediately, but I could also yield further up,
                        // to be resumed at a later time
                        ft.exec(&mut ctxt.rt).unwrap();
                    }
                    StepResult::Working(WhichToken::Ref(rtw)) => {
                        // The runtime yields back at every call to a "builtin". Here, I
                        // call the builtin immediately, but I could also yield further up,
                        // to be resumed at a later time

                        let c = ctxt
                            .dict
                            .data
                            .get(&rtw.tok)
                            .and_then(|n| n.inner.get(rtw.idx))
                            .map(|n| n.clone().word);

                        ctxt.rt.provide_seq_tok(c).unwrap();
                    }
                }
            }
        }

        assert_eq!(output, &ctxt.output());

        assert_eq!(0, ctxt.rt.data_stk.data().len());
        assert_eq!(0, ctxt.rt.ret_stk.data().len());
        assert_eq!(0, ctxt.rt.flow_stk.data().len());
    }
}

fn s(words: &str) -> Vec<String> {
    words.split_whitespace().map(str::to_string).collect()
}
