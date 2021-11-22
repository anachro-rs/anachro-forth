use anachro_forth_host::evaluate;
use std::io::Result as IoResult;
use std::io::{stdin, stdout, Write};

use anachro_forth_core::{Error, StepResult, WhichToken};
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
                Ok(StepResult::Working(WhichToken::Single(ft))) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time
                    ft.exec(&mut ctxt.rt).unwrap();
                }
                Ok(StepResult::Working(WhichToken::Ref(rtw))) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time

                    let c = ctxt.dict.data
                        .get(&rtw.tok)
                        .and_then(|n| n.inner.get(rtw.idx))
                        .map(|n| n.clone().word);

                    ctxt.rt.provide_seq_tok(c).unwrap();

                }
                Ok(StepResult::Done) => break true,
                Err(e) => {
                    eprintln!("ERROR! -> {:?}", e);
                    break false;
                }
            }
        };
        ctxt.dict.data.retain(|k, _| !k.starts_with("__"));
        let ser = ctxt.serialize();
        println!("{:?}", ser);

        let pcser = postcard::to_stdvec(&ser).unwrap();
        // println!("{:02X?}", pcser);

        let mut zc = rzcobs::encode(&pcser);
        zc.push(0);
        println!("{:02X?}", zc);

        let mut rler = kolben::rlercobs::encode(&pcser);
        rler.push(0);
        // println!("{:02X?}", rler);

        println!("{}, {}, {}", pcser.len(), zc.len(), rler.len());

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
