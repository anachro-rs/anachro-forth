use std::borrow::Cow;
use std::fs::{read_to_string, write};
use std::io::{stdin, stdout, Write};
use std::io::{Read, Result as IoResult};
use std::path::PathBuf;

use structopt::StructOpt;

use a4_core::compiler::Context;
use a4_core::std_rt::std_builtins;
use a4_core::{Error, StepResult, WhichToken};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "a4",
    about = "A forth-inspired, bytecode-compiled scripting language for Anachro Powerbus"
)]
enum Opt {
    /// Start an interactive "Read, Evaluate, Print, Loop" session
    Repl {
        /// A source file to initialize the repl with. Must be an ".a4" file
        input: Option<PathBuf>,

        #[structopt(short, long)]
        debug: bool,
    },

    /// Run a given ".fth" file, exiting after execution
    Run {
        input: PathBuf,

        #[structopt(short, long)]
        debug: bool,
    },

    /// Compile the provided ".fth" source file into an ".a4" compiled
    /// output
    Compile {
        /// The source file to compile
        input: PathBuf,

        /// The output compiled path. If none is provided, the input file
        /// path will be used, replacing the extension with ".a4"
        output: Option<PathBuf>,

        /// Omit the names of user-defined words from the serialized output
        /// This is useful for reducing bytes-on-the-wire
        #[structopt(short, long = "omit-word-names")]
        omit_word_names: bool,
    },
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();

    match opt {
        Opt::Repl { input, debug } => {
            println!("Entering Repl...");
            repl_main(input, debug)?;
        }
        Opt::Compile {
            input,
            output,
            omit_word_names,
        } => {
            let output = output.unwrap_or({
                let mut out = input.clone();
                assert!(out.set_extension("a4"), "no filename?");
                out
            });
            compile_main(input, output, omit_word_names)?;
        }
        Opt::Run { input, debug } => {
            run_main(input, debug)?;
        }
    }

    Ok(())
}

fn compile_main(input: PathBuf, output: PathBuf, omit_word_names: bool) -> Result<(), Error> {
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

    eprintln!(
        "
WARNING: Found at least one non-definition in the input file.
These line(s) will NOT be serialized or executed. Please review
your source file to ensure it ONLY includes definitions, which
start with a ':', and end with a ';'.
"
    );

    let mut ser = ctxt.serialize();

    if omit_word_names {
        ser.data_map = None;
    }

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

fn run_main(input: PathBuf, debug: bool) -> Result<(), Error> {
    let mut ctxt = Context::with_builtins(std_builtins());

    let input = read_to_string(input).map_err(|_| Error::Input)?;

    for line in input.lines() {
        let input: Vec<String> = line.split_whitespace().map(str::to_string).collect();

        if input.is_empty() {
            continue;
        }

        if debug {
            println!("=> {}", line);
        }

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
            if debug {
                println!("# {:?} - {:?}", ctxt.data_stack().data(), ctxt.return_stack().data());
            }
        };
        ctxt.dict.data.retain(|k, _| !k.starts_with("__"));
        print(&mut ctxt, is_ok);
    }

    Ok(())
}

fn repl_main(input: Option<PathBuf>, debug: bool) -> Result<(), Error> {
    let mut ctxt = Context::with_builtins(std_builtins());

    if let Some(pb) = input {
        match pb.extension().map(|x| x.to_string_lossy()) {
            Some(Cow::Borrowed("a4")) => {
                let mut f = std::fs::File::open(pb).unwrap();
                let mut buf = Vec::new();
                f.read_to_end(&mut buf).unwrap();
                assert_eq!(Some(&0x00), buf.last());
                buf.pop();
                let unrz = rzcobs::decode(&buf).unwrap();
                let deser = postcard::from_bytes(&unrz).unwrap();
                ctxt.load_ser_dict(&deser);
            }
            Some(_) => todo!("No .fth loading yet, sorry"),
            None => {
                eprintln!("ERROR: No extension found!");
                return Err(Error::InternalError);
            }
        }
    }

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
            if debug {
                println!("# {:?} - {:?}", ctxt.data_stack().data(), ctxt.return_stack().data());
            }
        };
        ctxt.dict.data.retain(|k, _| !k.starts_with("__"));
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
