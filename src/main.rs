use std::{collections::BTreeMap, io::{Write, stdin, stdout}};
use std::io::Result as IoResult;

#[derive(Clone)]
enum Word {
    Builtin(fn(&mut Stack, &mut Dict) -> Result<(), ()>),
    Uncompiled(Vec<String>),
}

type Stack = Vec<i32>;
type Dict = BTreeMap<String, Word>;

fn main() -> IoResult<()> {
    let mut stack: Stack = Vec::new();
    let mut dict: Dict = BTreeMap::new();

    for (word, func) in BUILT_IN_WORDS {
        dict.insert(word.to_string(), func.clone());
    }

    loop {
        let input = read()?;
        evaluate(&mut stack, &mut dict, input).unwrap();
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

fn evaluate(stack: &mut Stack, dict: &mut Dict, data: Vec<String>) -> Result<(), ()> {
    match (data.first(), data.last()) {
        (Some(f), Some(l)) if f == ":" && l == ";" => {

            // Must have ":", "$NAME", "$SOMETHING+", ";"
            assert!(data.len() >= 4);

            // TODO: Validate all words are valid at "compile" time!
            let name = data[1].to_lowercase();

            // TODO: Doesn't handle "empty" definitions
            let relevant = &data[2..][..data.len() - 3];

            dict.insert(name, Word::Uncompiled(relevant.to_vec()));
        }
        _ => {
            for word in data {
                if let Some(werd) = dict.get(&word.to_lowercase()).map(Clone::clone) {
                    match werd {
                        Word::Builtin(biw) => {
                            (biw)(stack, dict)?;
                        },
                        Word::Uncompiled(ucw) => {
                            evaluate(stack, dict, ucw.clone())?;
                        },
                    }
                } else if let Some(num) = parse_num(&word) {
                    stack.push(num);
                } else {
                    println!("");
                    panic!("Not found!");
                }
            }
        }
    }


    Ok(())
}

fn print() {
    println!(" ok ");
}

fn bi_emit(stack: &mut Stack, _dict: &mut Dict) -> Result<(), ()> {
    let word = stack.pop().ok_or(())? as u32;
    let symbol = std::char::from_u32(word).unwrap_or('‽');
    print!("{}", symbol);
    stdout().flush().map_err(drop)
}

fn bi_coredump(stack: &mut Stack, dict: &mut Dict) -> Result<(), ()> {
    println!("STACK:");
    println!("{:08X?}", stack);
    println!("");

    println!("DICT:");
    for (key, word) in dict {
        print!("  - {:?} => ", key);
        match word {
            Word::Builtin(_) => println!("(builtin)"),
            Word::Uncompiled(ucw) => println!("{:?}", ucw),
        }
    }

    Ok(())
}

fn bi_pop(stack: &mut Stack, _dict: &mut Dict) -> Result<(), ()> {
    println!("{}", stack.pop().ok_or(())?);
    Ok(())
}

fn bi_cr(_stack: &mut Stack, _dict: &mut Dict) -> Result<(), ()> {
    println!("");
    Ok(())
}

static BUILT_IN_WORDS: &[(&str, Word)] = &[
    ("emit", Word::Builtin(bi_emit)),
    (".", Word::Builtin(bi_pop)),
    ("cr", Word::Builtin(bi_cr)),

    // Debug
    ("coredump", Word::Builtin(bi_coredump)),
];


// TODO: Make tests!

/*
james@archx1c6g ➜  forth-hax git:(main) ✗ cargo run
   Compiling forth-hax v0.1.0 (/home/james/personal/forth-hax)
    Finished dev [unoptimized + debuginfo] target(s) in 0.47s
     Running `target/debug/forth-hax`
=> coredump
STACK:
[]

DICT:
  - "." => (builtin)
  - "coredump" => (builtin)
  - "cr" => (builtin)
  - "emit" => (builtin)
 ok
=> : STAR 42 EMIT ;
 ok
=> : STARS STAR STAR STAR STAR STAR CR ;
 ok
=> coredump
STACK:
[]

DICT:
  - "." => (builtin)
  - "coredump" => (builtin)
  - "cr" => (builtin)
  - "emit" => (builtin)
  - "star" => ["42", "EMIT"]
  - "stars" => ["STAR", "STAR", "STAR", "STAR", "STAR", "CR"]
 ok
=> stars
*****
 ok
=> ^C
*/
