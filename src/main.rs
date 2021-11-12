use std::io::Result as IoResult;
use std::io::{stdin, stdout, Write};

use forth_hax::builtins::BUILT_IN_WORDS;
use forth_hax::*;

fn main() -> IoResult<()> {
    let mut ctxt = Context::with_builtins(BUILT_IN_WORDS);

    loop {
        let input = read().unwrap();
        evaluate(&mut ctxt, input).unwrap();
        while let StepResult::Working = ctxt.step() {
            // ...
        }
        print(&mut ctxt);
    }
}

fn read() -> IoResult<Vec<String>> {
    print!("=> ");
    stdout().flush().ok();
    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap();

    Ok(buf.split_whitespace().map(str::to_string).collect())
}

fn print(ctxt: &mut Context) {
    print!("{}", ctxt.output());
    println!(" ok ");
}
