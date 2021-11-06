use std::sync::Arc;
use std::{collections::BTreeMap, ops::Deref};

pub mod builtins;

#[derive(Clone)]
pub enum Word {
    LiteralVal(i32),
    Builtin(fn(&mut Context) -> Result<(), ()>),
    Compiled(Vec<Arc<Word>>),
    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
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
    pub fn with_builtins(bi: &[(&str, Word)]) -> Self {
        let mut new = Context {
            data_stk: Vec::new(),
            ret_stk: Vec::new(),
            flow_stk: Vec::new(),
            dict: BTreeMap::new(),
        };

        for (word, func) in bi {
            new.dict.insert(word.to_string(), Arc::new(func.clone()));
        }

        new
    }

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
            }
            Word::Builtin(func) => {
                func(self).unwrap();
                None
            }
            Word::Compiled(words) => {
                let ret = words.get(cur.idx).map(Clone::clone);
                cur.idx += 1;
                ret
            }
            Word::UncondRelativeJump { offset } => {
                jump = Some(*offset);
                None
            }
            Word::CondRelativeJump { offset, jump_on } => {
                let topvar = self.data_stk.pop().unwrap();

                // Truth table:
                // tv == 0 | jump_on | jump.unwrap()
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
            println!("Jumping!");
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
        self.flow_stk.push(ExecCtx { idx: 0, word });
    }
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

    let lowered = data
        .iter()
        .map(String::as_str)
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let mut if_ct = 0;
    let mut else_ct = 0;
    let mut then_ct = 0;
    let mut do_ct = 0;
    let mut loop_ct = 0;

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
                    .ok_or(())
                    .unwrap();

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

                Arc::new(Word::CondRelativeJump {
                    offset,
                    jump_on: false,
                })
            }
            "else" => {
                // All we need to do on an else is insert an unconditional jump to the then.
                let offset = lowered
                    .iter()
                    .skip(idx)
                    .position(|w| w == "then")
                    .ok_or(())
                    .unwrap();

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
            "do" => {
                output.push(Arc::new(Word::Builtin(builtins::bi_retstk_push)));
                output.push(Arc::new(Word::Builtin(builtins::bi_retstk_push)));
                do_ct += 1;
                continue;
            }
            "loop" => {
                output.push(Arc::new(Word::LiteralVal(1)));
                output.push(Arc::new(Word::Builtin(builtins::bi_add)));
                output.push(Arc::new(Word::Builtin(builtins::bi_gt)));

                let mut count: usize = do_ct - loop_ct;
                let offset = lowered[..idx]
                    .iter()
                    .rev()
                    .position(|w| {
                        if w == "do" {
                            count = dbg!(dbg!(count).checked_sub(1).unwrap());
                        }

                        count == 0
                    })
                    .ok_or(())
                    .unwrap();

                loop_ct += 1;

                println!("{} => {}; {}", offset, idx, output.len());

                Arc::new(Word::CondRelativeJump {
                    offset: (-1i32 * offset as i32) - 2,
                    jump_on: true,
                })
            }

            // Now, check for "normal" words, e.g. numeric literals or dictionary words
            other => {
                if let Some(dword) = ctxt.dict.get(other).cloned() {
                    dword
                } else if let Some(num) = parse_num(other).map(Word::LiteralVal) {
                    Arc::new(num)
                } else {
                    panic!() // return Err(())
                }
            }
        };

        output.push(comp);
    }

    // TODO: This probably isn't SUPER robust, but for now is a decent sanity check
    // that we have properly paired if/then/elses
    if if_ct != then_ct {
        panic!() // return Err(());
    }
    if else_ct > if_ct {
        panic!() // return Err(());
    }

    Ok(output)
}

pub fn evaluate(ctxt: &mut Context, data: Vec<String>) -> Result<(), ()> {
    match (data.first(), data.last()) {
        (Some(f), Some(l)) if f == ":" && l == ";" => {
            // Must have ":", "$NAME", "$SOMETHING+", ";"
            assert!(data.len() >= 4);

            let name = data[1].to_lowercase();

            // TODO: Doesn't handle "empty" definitions
            let relevant = &data[2..][..data.len() - 3];

            let compiled = compile(ctxt, relevant).unwrap();

            ctxt.dict.insert(name, Arc::new(Word::Compiled(compiled)));
        }
        _ => {
            // We should interpret this as a line to compile and run
            // (but then discard, because it isn't bound in the dict)
            let temp_compiled = Arc::new(Word::Compiled(compile(ctxt, &data).unwrap()));
            ctxt.push_exec(temp_compiled);
        }
    }

    Ok(())
}
