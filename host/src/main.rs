use std::io::Result as IoResult;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::fs::{read_to_string, write};

use structopt::StructOpt;

use a4_core::compiler::Context;
use a4_core::std_rt::std_builtins;
use a4_core::{Error, StepResult, WhichToken};

#[derive(Debug, StructOpt)]
#[structopt(name = "a4", about = "A forth-inspired, bytecode-compiled scripting language for Anachro Powerbus")]
enum Opt {
    /// Start an interactive "Read, Evaluate, Print, Loop" session
    Repl,

    /// Compile the provided ".fth" source file into an ".a4" compiled
    /// output
    Compile {
        input: PathBuf,
        output: Option<PathBuf>,
    },
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();

    match opt {
        Opt::Repl => {
            println!("Entering Repl...");
            repl_main()?;
        }
        Opt::Compile { input, output } => {
            let output = output.unwrap_or({
                let mut out = input.clone();
                assert!(out.set_extension("a4"), "no filename?");
                out
            });
            compile_main(input, output)?;
        }
        _ => todo!(),
    }

    Ok(())
}

fn compile_main(input: PathBuf, output: PathBuf) -> Result<(), Error> {
    let mut ctxt = Context::with_builtins(std_builtins());

    let source = read_to_string(&input).map_err(|_| Error::Input)?;

    for line in source.lines() {
        let parts = line.split_whitespace().map(str::to_string).collect();
        ctxt.evaluate(parts)?;
    }

    let mut extras = false;
    ctxt.dict.data.retain(|k, _| {
        let keep = !k.starts_with("__");
        if !keep {
            extras = true;
        }
        keep
    });

    eprintln!("
WARNING: Found at least one non-definition in the input file.
These line(s) will NOT be serialized or executed. Please review
your source file to ensure it ONLY includes definitions, which
start with a ':', and end with a ';'.
");

    let ser = ctxt.serialize();

    let pcser = postcard::to_stdvec(&ser).unwrap();
    let mut zc = rzcobs::encode(&pcser);
    zc.push(0);

    write(&output, &zc).map_err(|_| Error::OutputFormat)?;

    println!("Input file:  {:?}", input);
    println!("Output file: {:?}", output);
    println!("===========================================");
    println!("Builtin words used:      {}", ser.bis.len());
    println!("User defined words:      {}", ser.data.len());
    println!("Serialized size (bytes): {}", zc.len());

    Ok(())
}

fn repl_main() -> Result<(), Error> {
    let mut ctxt = Context::with_builtins(std_builtins());

    loop {
        let input = read().map_err(|_| Error::Input)?;
        ctxt.evaluate(input)?;
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

                    let c = ctxt
                        .dict
                        .data
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
