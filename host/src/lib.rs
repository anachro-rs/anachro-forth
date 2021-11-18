use std::collections::BTreeSet;
use std::sync::Arc;
use std::{collections::BTreeMap, ops::Deref};
use serde::Serialize;

use anachro_forth_core as afc;

use afc::{Error, Stack, StepResult, ExecStack, RuntimeSeqCtx, RefWord, Runtime};

pub mod builtins;

#[derive(Clone)]
pub enum Word<Sdata, Sexec> {
    LiteralVal(i32),
    Builtin {
        name: &'static str,
        func: fn(&mut Runtime<Sdata, Sexec>) -> Result<(), Error>,
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

impl<Sdata, Sexec> Word<Sdata, Sexec> {
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

pub struct ExecCtx<Sdata, Sexec> {
    idx: usize,
    word: Arc<Word<Sdata, Sexec>>,
}

pub struct Dict {
    pub(crate) data: BTreeMap<String, Arc<Word<Sdata, Sexec>>>,
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
pub struct StdVecStack<T> {
    data: Vec<T>,
    err: Error,
}

impl<T> StdVecStack<T> {
    pub fn new(err: Error) -> Self {
        StdVecStack {
            data: Vec::new(),
            err,
        }
    }
}

impl<T> Stack for StdVecStack<T> {
    type Item = T;

    fn push(&mut self, data: T) {
        self.data.push(data);
    }

    fn pop(&mut self) -> Result<T, Error> {
        self.data.pop().ok_or(Error::DataStackUnderflow)
    }

    fn last(&self) -> Result<&T, Error> {
        self.data.last().ok_or(self.err.clone())
    }

    fn last_mut(&mut self) -> Result<&mut T, Error> {
        self.data.last_mut().ok_or(self.err.clone())
    }

    fn get_mut(&mut self, index: usize) -> Result<&mut T, Error> {
        self.data.get_mut(index).ok_or(self.err.clone())
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

struct FlowBoi<'a> {
    flow_stk: &'a mut StdVecStack<ExecCtx<StdVecStack<i32>, FlowBoi<'a>>>,
    dict: &'a mut Dict,
}

impl<'a> From<RuntimeSeqCtx<'a, StdVecStack<i32>, FlowBoi<'a>>> for ExecCtx<StdVecStack<i32>, FlowBoi<'a>> {
    fn from(other: RuntimeSeqCtx<'a, StdVecStack<i32>, FlowBoi<'a>>) -> ExecCtx {
        ExecCtx {
            idx: other.idx,
            word: Arc::new(match other.word {
                RefWord::LiteralVal(val) => Word::LiteralVal(val),
                RefWord::Builtin { name, func } => Word::Builtin {
                    func, name
                },
                RefWord::Compiled { name, data } => todo!(),
                RefWord::UncondRelativeJump { offset } => Word::UncondRelativeJump { offset },
                RefWord::CondRelativeJump { offset, jump_on } => Word::CondRelativeJump { offset, jump_on },
            })
        }
    }
}

impl<'a> ExecStack<'a, StdVecStack<i32>> for FlowBoi<'a> {
    fn push(&mut self, data: RuntimeSeqCtx<'a, StdVecStack<i32>, Self>) {
        self.flow_stk.data.push(data.into());
    }

    fn pop(&mut self) -> Result<RuntimeSeqCtx<'a, StdVecStack<i32>, Self>, Error> {
        // self.data.pop().ok_or(Error::DataStackUnderflow).map(|d| {
        //     todo!()
        // })
        todo!()
    }

    fn last(&self) -> Result<&RuntimeSeqCtx<'a, StdVecStack<i32>, Self>, Error> {
        // self.data.last().ok_or(self.err.clone()).map(|d| todo!())
        todo!()
    }

    fn last_mut(&mut self) -> Result<&mut RuntimeSeqCtx<'a, StdVecStack<i32>, Self>, Error> {
        // self.data.last_mut().ok_or(self.err.clone()).map(|d| todo!())
        todo!()
    }

    fn get_mut(&mut self, index: usize) -> Result<&mut RuntimeSeqCtx<'a, StdVecStack<i32>, Self>, Error> {
        // self.data.get_mut(index).ok_or(self.err.clone()).map(|d| todo!())
        todo!()
    }

    fn len(&self) -> usize {
        // self.data.len()
        todo!()
    }
}

pub struct Context {
    data_stk: StdVecStack<i32>,
    ret_stk: StdVecStack<i32>,
    flow_stk: StdVecStack<ExecCtx>,
    dict: Dict,
    cur_output: String,
}

impl Context {
    pub fn serialize(&self) -> SerDict {
        let mut dict = self.dict.serialize();

        let mut bi_set = BTreeSet::new();

        for (_key, var) in dict.data.iter() {
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

    pub fn step(&mut self) -> Result<StepResult, Error> {
        let mut lol = afc::Runtime {
            data_stk: &mut self.data_stk,
            flow_stk: &mut FlowBoi {
                flow_stk: &mut self.flow_stk,
                dict: &mut self.dict,
            },
            ret_stk: &mut self.ret_stk,
        };

        todo!()
    }

    pub fn data_stack(&self) -> &StdVecStack<i32> {
        &self.data_stk
    }

    pub fn return_stack(&self) -> &StdVecStack<i32> {
        &self.ret_stk
    }

    pub fn flow_stack(&self) -> &StdVecStack<ExecCtx> {
        &self.flow_stk
    }

    pub fn with_builtins(bi: &[(&'static str, fn(&mut Context) -> Result<(), Error>)]) -> Self {
        let mut new = Context {
            data_stk: StdVecStack::new(Error::DataStackEmpty),
            ret_stk: StdVecStack::new(Error::RetStackEmpty),
            flow_stk: StdVecStack::new(Error::FlowStackEmpty),
            dict: Dict::new(),
            cur_output: String::new(),
        };

        for (word, func) in bi {
            new.dict.data.insert(word.to_string(), Arc::new(Word::Builtin { name: word, func: *func }));
        }

        new
    }

    pub fn output(&mut self) -> String {
        let mut new_out = String::new();
        core::mem::swap(&mut self.cur_output, &mut new_out);
        new_out
    }

    pub fn push_exec(&mut self, word: Arc<Word>) {
        Stack::push(&mut self.flow_stk, ExecCtx { idx: 0, word });
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

                    _ => return Err(Error::InternalError),
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
                    return Err(Error::InternalError);
                }
            }
        };

        output.push(comp);
    }

    // TODO: This probably isn't SUPER robust, but for now is a decent sanity check
    // that we have properly paired if/then/elses
    if if_ct != then_ct {
        return Err(Error::InternalError);
    }
    if else_ct > if_ct {
        return Err(Error::InternalError);
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
