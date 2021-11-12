use std::io::Result as IoResult;
use std::io::{stdin, stdout, Write};

use forth_hax::builtins::BUILT_IN_WORDS;
use forth_hax::*;

fn main() -> Result<(), forth_hax::Error> {
    let mut ctxt = Context::with_builtins(BUILT_IN_WORDS);

    loop {
        let input = read().map_err(|_| Error::Input)?;
        evaluate(&mut ctxt, input)?;
        while let Ok(StepResult::Working) = ctxt.step() {
            // ...
        }
        print(&mut ctxt);
    }
}

fn read() -> IoResult<Vec<String>> {
    print!("=> ");
    stdout().flush().ok();
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;

    Ok(buf.split_whitespace().map(str::to_string).collect())
}

fn print(ctxt: &mut Context) {
    print!("{}", ctxt.output());
    println!(" ok ");
}
