use std::io::Result as IoResult;
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    io::{stdin, stdout, Write},
    ops::Deref,
};

#[derive(Clone)]
pub enum Word {
    LiteralVal(i32),
    Builtin(fn(&mut Context) -> Result<(), ()>),
    Compiled(Vec<Arc<Word>>),
    UncondRelativeJump {
        offset: i32,
    },
    CondRelativeJump {
        offset: i32,
        jump_on: bool,
    },
}

struct ExecCtx {
    idx: usize,
    word: Arc<Word>,
}

type Stack = Vec<i32>;
type Dict = BTreeMap<String, Arc<Word>>;

pub struct Context {
    data_stk: Stack,
    ret_stk: Stack,
    flow_stk: Vec<ExecCtx>,
    dict: Dict,
}

pub enum StepResult {
    Done,
    Working,
}

impl Context {
    pub fn step(&mut self) -> StepResult {
        let cur = match self.flow_stk.last_mut() {
            Some(frame) => frame,
            None => return StepResult::Done,
        };

        let mut jump = None;

        let word: &Word = cur.word.deref();

        let to_push = match word {
            Word::LiteralVal(lit) => {
                self.data_stk.push(*lit);
                None
            },
            Word::Builtin(func) => {
                func(self).unwrap();
                None
            },
            Word::Compiled(words) => {
                let ret = words.get(cur.idx).map(Clone::clone);
                cur.idx += 1;
                ret
            },
            Word::UncondRelativeJump { offset } => {
                jump = Some(*offset);
                None
            }
            Word::CondRelativeJump { offset, jump_on } => {
                let topvar = self.data_stk.pop().unwrap();

                // Truth table:
                // tv == 0 | jump_on | jump?
                // ========|=========|=======
                // false   | false   | no
                // true    | false   | yes
                // false   | true    | yes
                // true    | true    | no

                let do_jump = (topvar == 0) ^ jump_on;

                if do_jump {
                    jump = Some(*offset);
                }
                None
            }
        };

        if let Some(push) = to_push {
            self.push_exec(push);
        } else {
            self.flow_stk.pop();
        }

        if let Some(jump) = jump {
            // We just popped off the jump command, so now we are back in
            // the "parent" frame.

            let new_cur = self.flow_stk.last_mut().unwrap();

            if jump < 0 {
                let abs = jump.abs() as usize;

                assert!(abs <= new_cur.idx);

                new_cur.idx -= abs;
            } else {
                let abs = jump as usize;
                assert_ne!(abs, 0);
                new_cur.idx = new_cur.idx.checked_add(abs).unwrap();
            }
        }

        StepResult::Working
    }

    pub fn push_exec(&mut self, word: Arc<Word>) {
        self.flow_stk.push(ExecCtx {
            idx: 0,
            word,
        });
    }
}

fn main() -> IoResult<()> {
    let mut ctxt = Context {
        data_stk: Vec::new(),
        ret_stk: Vec::new(),
        flow_stk: Vec::new(),
        dict: BTreeMap::new(),
    };

    for (word, func) in BUILT_IN_WORDS {
        ctxt.dict.insert(word.to_string(), Arc::new(func.clone()));
    }

    loop {
        let input = read()?;
        evaluate(&mut ctxt, input).unwrap();
        while let StepResult::Working = ctxt.step() {
            // ...
        }
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
//
// See https://github.com/rust-analyzer/rust-analyzer/blob/c96481e25f08d1565cb9b3cac89323216e6f8d7f/crates/syntax/src/ast/token_ext.rs#L616-L662
// for one way of doing this!
fn parse_num(input: &str) -> Option<i32> {
    input.parse::<i32>().ok()
}

fn compile(ctxt: &mut Context, data: &[String]) -> Result<Vec<Arc<Word>>, ()> {
    let mut output = Vec::new();

    let lowered = data.iter().map(String::as_str).map(str::to_lowercase).collect::<Vec<_>>();
    let mut if_ct = 0;
    let mut else_ct = 0;
    let mut then_ct = 0;

    for (idx, d) in lowered.iter().enumerate() {
        let comp = match d.as_str() {
            // First, check for any "Magical" words that do not appear in the dictionary, and need to
            // be handled in a special way
            "if" => {
                // Seek forward to find the then/else
                let offset = lowered
                    .iter()
                    .skip(idx)
                    .position(|w| ["then", "else"].contains(&w.as_str()))
                    .ok_or(()).unwrap();

                if_ct += 1;

                let offset = match lowered[idx + offset].as_str() {
                    // We have to compensate that "then" doesn't actually
                    // appear in the compiled output
                    "then" => offset - 1,

                    // Here, there is no "then", but we do have to compensate
                    // for the unconditional jump that appears where else appears
                    "else" => offset,

                    _ => panic!(),
                } as i32;

                Arc::new(Word::CondRelativeJump { offset, jump_on: false })
            }
            "else" => {
                // All we need to do on an else is insert an unconditional jump to the then.
                let offset = lowered
                    .iter()
                    .skip(idx)
                    .position(|w| w == "then")
                    .ok_or(()).unwrap();

                // Note: Balance check handled later
                else_ct += 1;

                // We have to compensate that "then" doesn't actually
                // appear in the compiled output
                let offset = offset as i32 - 1;

                Arc::new(Word::UncondRelativeJump { offset })
            }
            "then" => {
                then_ct += 1;
                // For now, we only using 'then' as a sentinel value for if/else
                continue;
            }

            // Now, check for "normal" words, e.g. numeric literals or dictionary words
            other => {
                if let Some(dword) = ctxt.dict.get(other).cloned() {
                    dword
                } else if let Some(num) = parse_num(other).map(Word::LiteralVal) {
                    Arc::new(num)
                } else {
                    return Err(())
                }
            }
        };

        output.push(comp);
    }

    // TODO: This probably isn't SUPER robust, but for now is a decent sanity check
    // that we have properly paired if/then/elses
    if if_ct != then_ct {
        panic!("{} {}", if_ct, then_ct);
        return Err(());
    }
    if else_ct > if_ct {
        panic!();
        return Err(());
    }

    Ok(output)
}

fn evaluate(ctxt: &mut Context, data: Vec<String>) -> Result<(), ()> {
    match (data.first(), data.last()) {
        (Some(f), Some(l)) if f == ":" && l == ";" => {
            // Must have ":", "$NAME", "$SOMETHING+", ";"
            assert!(data.len() >= 4);

            let name = data[1].to_lowercase();

            // TODO: Doesn't handle "empty" definitions
            let relevant = &data[2..][..data.len() - 3];

            let compiled = compile(ctxt, relevant)?;

            ctxt.dict.insert(name, Arc::new(Word::Compiled(compiled)));
        }
        _ => {
            // We should interpret this as a line to compile and run
            // (but then discard, because it isn't bound in the dict)
            let temp_compiled = Arc::new(
                Word::Compiled(compile(ctxt, &data)?)
            );
            ctxt.push_exec(temp_compiled);
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
            Word::CondRelativeJump { .. } => println!("COND RELATIVE JUMP! TODO!"),
            Word::UncondRelativeJump { .. } => println!("UNCOND RELATIVE JUMP! TODO!"),
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

fn bi_jf2(ctxt: &mut Context) -> Result<(), ()> {
    // So, we need to push the counter of the SECOND
    // item on the stack forward two items
    let flow_len = ctxt.flow_stk.len();

    // We must have the current item (this function), AND
    // a parent
    assert!(flow_len >= 2);
    let parent = ctxt.flow_stk.get_mut(flow_len - 2).ok_or(())?;

    parent.idx += 2;

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

fn bi_eq(ctxt: &mut Context) -> Result<(), ()> {
    let val1 = ctxt.data_stk.pop().ok_or(())?;
    let val2 = ctxt.data_stk.pop().ok_or(())?;
    ctxt.data_stk.push(if val1 == val2 { -1 } else { 0 });
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
    ("=", Word::Builtin(bi_eq)),

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

    // Test item to verify control stack
    ("jf2", Word::Builtin(bi_jf2)),
];
