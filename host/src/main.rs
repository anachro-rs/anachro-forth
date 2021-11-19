use anachro_forth_host::evaluate;
use std::io::Result as IoResult;
use std::io::{stdin, stdout, Write};

use anachro_forth_core::{Error, StepResult};
use anachro_forth_host::Context;
use anachro_forth_core::std_rt::std_builtins;

fn main() -> Result<(), Error> {
    let mut ctxt = Context::with_builtins(
        std_builtins()
    );

    loop {
        let input = read().map_err(|_| Error::Input)?;
        evaluate(&mut ctxt, input)?;
        let is_ok = loop {
            match ctxt.step() {
                Ok(StepResult::Working(f)) => {
                    f.exec(&mut ctxt.rt)?;
                }
                Ok(StepResult::Done) => break true,
                Err(e) => {
                    eprintln!("ERROR! -> {:?}", e);
                    break false;
                }
            }
        };
        let ser = ctxt.serialize();
        println!("{:?}", ser);
        let pcser = postcard::to_stdvec(&ser).unwrap();
        println!("{:02X?}", pcser);
        println!("{}", pcser.len());
        print(&mut ctxt, is_ok);
    }
}

fn read() -> IoResult<Vec<String>> {
    print!("=> ");
    stdout().flush().ok();
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;

    Ok(buf.split_whitespace().map(str::to_string).collect())
}

fn print(ctxt: &mut Context, good: bool) {
    print!("{}", ctxt.output());
    if good {
        println!(" ok ");
    } else {
        println!(" bad ");
    }
}
