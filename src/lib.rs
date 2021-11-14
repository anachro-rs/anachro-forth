use std::collections::BTreeSet;
use std::sync::Arc;
use std::{collections::BTreeMap, ops::Deref};
use serde::{Serialize, Deserialize};

pub mod builtins;

#[derive(Debug, Clone)]
pub enum Error {
    /// Failed to write to the "stdout" style output
    OutputFormat,

    /// Failed to read from the "stdin" style input
    Input,

    /// Data stack underflowed
    DataStackUnderflow,

    /// Data stack was empty
    DataStackEmpty,

    /// Return stack was empty
    RetStackEmpty,

    /// Flow/Execution stack was empty
    FlowStackEmpty,

    /// Some kind of checked math failed
    BadMath,

    /// We found an "if" without an appropriate pair
    MissingIfPair,

    /// We found an "else" without an appropriate pair
    MissingElsePair,

    /// We found a "loop" without an appropriate pair
    MissingLoopPair,

    /// We found a "do" without an appropriate pair
    MissingDoPair,
}

impl From<core::fmt::Error> for Error {
    fn from(_other: core::fmt::Error) -> Self {
        Self::OutputFormat
    }
}

#[derive(Clone)]
pub enum Word {
    LiteralVal(i32),
    Builtin {
        name: &'static str,
        func: fn(&mut Context) -> Result<(), Error>,
    },
    Compiled {
        name: String,
        data: Vec<Arc<Word>>,
    },
    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

#[derive(Debug, Serialize)]
pub enum SerWord {
    LiteralVal(i32),
    Builtin {
        name: &'static str,
    },
    CompiledDefn(Vec<SerWord>),
    CompiledRef(String),
    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

impl Word {
    fn serialize(&self, toplevel: bool) -> SerWord {
        match self {
            Word::LiteralVal(lit) => SerWord::LiteralVal(*lit),
            Word::Builtin{ name, .. } => SerWord::Builtin { name },
            Word::Compiled { name, data: comp } => {
                if toplevel {
                    let mut out = Vec::new();
                    for c in comp.iter() {
                        out.push(c.serialize(false));
                    }
                    SerWord::CompiledDefn(out)
                } else {
                    SerWord::CompiledRef(name.clone())
                }
            },
            Word::UncondRelativeJump { offset } => SerWord::UncondRelativeJump { offset: *offset },
            Word::CondRelativeJump { offset, jump_on } => SerWord::CondRelativeJump { offset: *offset, jump_on: *jump_on },
        }
    }
}

pub struct ExecCtx {
    idx: usize,
    word: Arc<Word>,
}

pub struct Dict {
    pub(crate) data: BTreeMap<String, Arc<Word>>,
}

#[derive(Debug, Serialize)]
pub struct SerDict {
    data: BTreeMap<String, SerWord>,
}

impl Dict {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn serialize(&self) -> SerDict {
        let new = self.clone();
        let mut out = SerDict {
            data: BTreeMap::new()
        };

        for (word, val) in new.data.iter() {
            out.data.insert(word.clone(), val.serialize(true));
        }

        out
    }
}

#[derive(Debug)]
pub struct Stack {
    data: Vec<i32>,
    err: Error,
}

impl Stack {
    pub fn new(err: Error) -> Self {
        Stack {
            data: Vec::new(),
            err,
        }
    }

    pub fn push(&mut self, data: i32) {
        self.data.push(data);
    }

    pub fn pop(&mut self) -> Result<i32, Error> {
        self.data.pop().ok_or(Error::DataStackUnderflow)
    }

    pub fn last(&self) -> Result<&i32, Error> {
        self.data.last().ok_or(self.err.clone())
    }

    pub fn get_mut(&mut self, index: usize) -> Result<&mut i32, Error> {
        self.data.get_mut(index).ok_or(self.err.clone())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct Context {
    data_stk: Stack,
    ret_stk: Stack,
    flow_stk: Vec<ExecCtx>,
    dict: Dict,
    cur_output: String,
}

pub enum StepResult {
    Done,
    Working,
}

impl Context {
    pub fn serialize(&self) -> SerDict {
        let mut dict = self.dict.serialize();

        let mut bi_set = BTreeSet::new();

        for (key, var) in dict.data.iter() {
            if let SerWord::CompiledDefn(cwrd) = var {
                for w in cwrd {
                    if let SerWord::Builtin { name } = w {
                        bi_set.insert(*name);
                    }
                }
            }
        }

        dict.data.retain(|_k, v| {
            match v {
                SerWord::LiteralVal(_) => true,
                SerWord::Builtin { .. } => false,
                SerWord::CompiledDefn(_) => true,
                SerWord::CompiledRef(_) => todo!(),
                SerWord::UncondRelativeJump { .. } => true,
                SerWord::CondRelativeJump { .. } => true,
            }
        });

        dict
    }

    pub fn data_stack(&self) -> &Stack {
        &self.data_stk
    }

    pub fn return_stack(&self) -> &Stack {
        &self.ret_stk
    }

    pub fn flow_stack(&self) -> &[ExecCtx] {
        &self.flow_stk
    }

    pub fn with_builtins(bi: &[(&'static str, fn(&mut Context) -> Result<(), Error>)]) -> Self {
        let mut new = Context {
            data_stk: Stack::new(Error::DataStackEmpty),
            ret_stk: Stack::new(Error::RetStackEmpty),
            flow_stk: Vec::new(),
            dict: Dict::new(),
            cur_output: String::new(),
        };

        for (word, func) in bi {
            new.dict.data.insert(word.to_string(), Arc::new(Word::Builtin { name: word, func: *func }));
        }

        new
    }

    pub fn step(&mut self) -> Result<StepResult, Error> {
        match self.step_inner() {
            Ok(r) => Ok(r),
            Err(e) => {
                self.flow_stk.clear();
                while let Ok(_) = self.data_stk.pop() {}
                while let Ok(_) = self.ret_stk.pop() {}
                Err(e)
            }
        }
    }

    fn step_inner(&mut self) -> Result<StepResult, Error> {
        let cur = match self.flow_stk.last_mut() {
            Some(frame) => frame,
            None => return Ok(StepResult::Done),
        };

        let mut jump = None;

        let word: &Word = cur.word.deref();

        let to_push = match word {
            Word::LiteralVal(lit) => {
                self.data_stk.push(*lit);
                None
            }
            Word::Builtin { func, .. } => {
                func(self)?;
                None
            }
            Word::Compiled { name, data: words } => {
                let ret = words.get(cur.idx).map(Clone::clone);
                cur.idx += 1;
                ret
            }
            Word::UncondRelativeJump { offset } => {
                jump = Some(*offset);
                None
            }
            Word::CondRelativeJump { offset, jump_on } => {
                let topvar = self.data_stk.pop()?;

                // Truth table:
                // tv == 0 | jump_on | jump
                // ========|=========|=======
                // false   | false   | no
                // true    | false   | yes
                // false   | true    | yes
                // true    | true    | no
                let do_jump = (topvar == 0) ^ jump_on;

                println!("topvar: {}, jump_on: {}", topvar, jump_on);

                if do_jump {
                    println!("Jumping!");
                    jump = Some(*offset);
                } else {
                    println!("Not Jumping!");
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

            let new_cur = self.flow_stk.last_mut().ok_or(Error::FlowStackEmpty)?;

            if jump < 0 {
                let abs = jump.abs() as usize;

                assert!(abs <= new_cur.idx);

                new_cur.idx -= abs;
            } else {
                let abs = jump as usize;
                assert_ne!(abs, 0);
                new_cur.idx = new_cur.idx.checked_add(abs).ok_or(Error::BadMath)?;
            }
        }

        Ok(StepResult::Working)
    }

    pub fn push_exec(&mut self, word: Arc<Word>) {
        self.flow_stk.push(ExecCtx { idx: 0, word });
    }

    pub fn output(&mut self) -> String {
        let mut new_out = String::new();
        core::mem::swap(&mut self.cur_output, &mut new_out);
        new_out
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

fn compile(ctxt: &mut Context, data: &[String]) -> Result<Vec<Arc<Word>>, Error> {
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
                    .ok_or(Error::MissingIfPair)?;

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
                    .ok_or(Error::MissingElsePair)?;

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
                output.push(Arc::new(Word::Builtin { name: ">r", func: builtins::bi_retstk_push }));
                output.push(Arc::new(Word::Builtin { name: ">r", func: builtins::bi_retstk_push }));
                do_ct += 1;
                continue;
            }
            "loop" => {
                output.push(Arc::new(Word::Builtin { name: "PRIV_LOOP", func: builtins::bi_priv_loop }));

                let mut count: usize = do_ct - loop_ct;
                let offset = lowered[..idx]
                    .iter()
                    .rev()
                    .position(|w| {
                        if w == "do" {
                            if let Some(amt) = count.checked_sub(1) {
                                count = amt;
                            } else {
                                return false;
                            }
                        }

                        count == 0
                    })
                    .ok_or(Error::MissingLoopPair)?;

                loop_ct += 1;

                Arc::new(Word::CondRelativeJump {
                    offset: (-1i32 * offset as i32) - 2,
                    jump_on: false,
                })
            }

            // Now, check for "normal" words, e.g. numeric literals or dictionary words
            other => {
                if let Some(dword) = ctxt.dict.data.get(other).cloned() {
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

pub fn evaluate(ctxt: &mut Context, data: Vec<String>) -> Result<(), Error> {
    match (data.first(), data.last()) {
        (Some(f), Some(l)) if f == ":" && l == ";" => {
            // Must have ":", "$NAME", "$SOMETHING+", ";"
            assert!(data.len() >= 4);

            let name = data[1].to_lowercase();

            // TODO: Doesn't handle "empty" definitions
            let relevant = &data[2..][..data.len() - 3];

            let compiled = compile(ctxt, relevant)?;

            ctxt.dict.data.insert(name.clone(), Arc::new(Word::Compiled {
                name,
                data: compiled,
            }));
        }
        _ => {
            // We should interpret this as a line to compile and run
            // (but then discard, because it isn't bound in the dict)
            let temp_compiled = Arc::new(Word::Compiled { name: "_".into(), data: compile(ctxt, &data)? });
            ctxt.push_exec(temp_compiled);
        }
    }

    Ok(())
}
