use std::io::Result as IoResult;
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    io::{stdin, stdout, Write},
    ops::Deref,
};

#[derive(Clone)]
enum Word {
    LiteralVal(i32),
    Builtin(fn(&mut Context) -> Result<(), ()>),
    Compiled(Vec<Arc<Word>>),
}

impl Word {
    fn execute(&self, ctxt: &mut Context) -> Result<(), ()> {
        match self {
            Word::LiteralVal(lit) => {
                ctxt.data_stk.push(*lit);
                Ok(())
            }
            Word::Builtin(func) => func(ctxt),
            Word::Compiled(words) => {
                for word in words {
                    word.execute(ctxt)?;
                }
                Ok(())
            }
        }
    }
}

type Stack = Vec<i32>;
type Dict = BTreeMap<String, Arc<Word>>;

pub struct Context {
    data_stk: Stack,
    ret_stk: Stack,
    dict: Dict,
}

fn main() -> IoResult<()> {
    let mut ctxt = Context {
        data_stk: Vec::new(),
        ret_stk: Vec::new(),
        dict: BTreeMap::new(),
    };

    for (word, func) in BUILT_IN_WORDS {
        ctxt.dict.insert(word.to_string(), Arc::new(func.clone()));
    }

    loop {
        let input = read()?;
        evaluate(&mut ctxt, input).unwrap();
        print();
    }
}

fn read() -> IoResult<Vec<String>> {
    print!("=> ");
    stdout().flush().ok();
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;

    Ok(buf.split_whitespace().map(str::to_string).collect())
}

// TODO: Expand number parser
// Make this a function to later allow for more custom parsing
// of literals like '0b1111_0000_1111_0000'
fn parse_num(input: &str) -> Option<i32> {
    input.parse::<i32>().ok()
}

fn compile(ctxt: &mut Context, data: &[String]) -> Result<Vec<Arc<Word>>, ()> {
    data.iter()
        .map(|w| {
            ctxt.dict
                .get(&w.to_lowercase())
                .cloned()
                .or_else(|| parse_num(w).map(Word::LiteralVal).map(Arc::new))
        })
        .collect::<Option<Vec<Arc<Word>>>>()
        .ok_or(())
}

fn evaluate(ctxt: &mut Context, data: Vec<String>) -> Result<(), ()> {
    match (data.first(), data.last()) {
        (Some(f), Some(l)) if f == ":" && l == ";" => {
            // Must have ":", "$NAME", "$SOMETHING+", ";"
            assert!(data.len() >= 4);

            // TODO: Validate all words are valid at "compile" time!
            let name = data[1].to_lowercase();

            // TODO: Doesn't handle "empty" definitions
            let relevant = &data[2..][..data.len() - 3];

            let compiled = compile(ctxt, relevant)?;

            ctxt.dict.insert(name, Arc::new(Word::Compiled(compiled)));
        }
        _ => {
            for word in compile(ctxt, &data)?.iter() {
                word.execute(ctxt)?;
            }
        }
    }

    Ok(())
}

fn print() {
    println!(" ok ");
}

fn bi_emit(ctxt: &mut Context) -> Result<(), ()> {
    let word = ctxt.data_stk.pop().ok_or(())? as u32;
    let symbol = std::char::from_u32(word).unwrap_or('‽');
    print!("{}", symbol);
    stdout().flush().map_err(drop)
}

fn bi_coredump(ctxt: &mut Context) -> Result<(), ()> {
    println!("DATA STACK:");
    println!("{:08X?}", ctxt.data_stk);
    println!("");

    println!("RETURN/CONTROL STACK:");
    println!("{:08X?}", ctxt.ret_stk);
    println!("");

    println!("DICT:");
    for (key, word) in ctxt.dict.iter() {
        print!("  - {:?} => ", key);
        let word: &Word = word.deref();
        match word {
            Word::Builtin(_) => println!("(builtin)"),
            Word::Compiled(ucw) => println!("(compiled, len: {})", ucw.len()),
            Word::LiteralVal(lit) => println!("Literal: {}", lit),
        }
    }

    Ok(())
}

fn bi_pop(ctxt: &mut Context) -> Result<(), ()> {
    println!("{}", ctxt.data_stk.pop().ok_or(())?);
    Ok(())
}

fn bi_cr(_ctxt: &mut Context) -> Result<(), ()> {
    println!("");
    Ok(())
}

fn bi_retstk_push(ctxt: &mut Context) -> Result<(), ()> {
    let val = ctxt.data_stk.pop().ok_or(())?;
    ctxt.ret_stk.push(val);
    Ok(())
}

fn bi_retstk_pop(ctxt: &mut Context) -> Result<(), ()> {
    let val = ctxt.ret_stk.pop().ok_or(())?;
    ctxt.data_stk.push(val);
    Ok(())
}

fn bi_serdump(ctxt: &mut Context) -> Result<(), ()> {
    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::LiteralVal(val) = word {
            println!(
                "LIT\t{}\t0x{:08X}\t0x{:016X}",
                name, *val, word as *const _ as usize
            );
        }
    }

    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::Builtin(_) = word {
            println!("BLT\t{}\t{:016X}", name, word as *const _ as usize);
        }
    }

    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::Compiled(words) = word {
            println!(
                "CMP\t{}\t{:016X}\t{:016X?}",
                name,
                word as *const _ as usize,
                words
                    .iter()
                    .map(|w| {
                        // TODO: I should probably directly print literals
                        let word: &Word = w.deref();
                        word as *const _ as usize
                    })
                    .collect::<Vec<_>>()
            );
        }
    }

    Ok(())
}

static BUILT_IN_WORDS: &[(&str, Word)] = &[
    ("emit", Word::Builtin(bi_emit)),
    (".", Word::Builtin(bi_pop)),
    ("cr", Word::Builtin(bi_cr)),
    (">r", Word::Builtin(bi_retstk_push)),
    ("r>", Word::Builtin(bi_retstk_pop)),
    // TODO: This requires the ability to modify the input stream!
    //
    // This is supposed to return the address of the NEXT word in the
    // input stream
    //
    // ("'", Word::Builtin(bi_tick))

    // ( @ is the "load operator )  ok
    // ( ! is the "store operator" )  ok

    // Constants store a VALUE in the dict, which will be pushed on the stack
    //
    // I *think*:
    //
    // 5 CONSTANT X
    //
    // is equivalent to:
    //
    // : X 5 ;

    // Variables store the value, and put the ADDRESS on the stack when invoked
    //
    // I *think*:
    //
    // 0 VARIABLE ZERO
    //
    // is equvalent to:
    //
    // ???

    // Debug
    ("serdump", Word::Builtin(bi_serdump)),
    ("coredump", Word::Builtin(bi_coredump)),
];
