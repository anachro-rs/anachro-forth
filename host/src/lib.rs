// use std::collections::BTreeSet;
use std::sync::Arc;
use std::collections::BTreeMap;
use serde::Serialize;

use afc::std_rt::{BuiltinToken, Hax, StdFuncSeq, StdRuntimeWord, new_runtime};
use afc::{RuntimeWord, StepResult};
use anachro_forth_core as afc;

use afc::{std_rt::StdRuntime, Error};


#[derive(Debug, Serialize)]
pub enum SerWord {
    LiteralVal(i32),
    Builtin {
        name: &'static str,
    },
    Verb(String),
    VerbSeq(Vec<SerWord>),
    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

// impl<Sdata, Sexec> Word<Sdata, Sexec> {
//     fn serialize(&self, toplevel: bool) -> SerWord {
//         match self {
//             Word::LiteralVal(lit) => SerWord::LiteralVal(*lit),
//             Word::Builtin{ name, .. } => SerWord::Builtin { name },
//             Word::Compiled { name, data: comp } => {
//                 if toplevel {
//                     let mut out = Vec::new();
//                     for c in comp.iter() {
//                         out.push(c.serialize(false));
//                     }
//                     SerWord::CompiledDefn(out)
//                 } else {
//                     SerWord::CompiledRef(name.clone())
//                 }
//             },
//             Word::UncondRelativeJump { offset } => SerWord::UncondRelativeJump { offset: *offset },
//             Word::CondRelativeJump { offset, jump_on } => SerWord::CondRelativeJump { offset: *offset, jump_on: *jump_on },
//         }
//     }
// }

// TODO: Is this redundant, now that every word has a name already?
#[derive(Clone)]
pub struct NamedStdRuntimeWord {
    name: String,
    word: StdRuntimeWord,
}

impl NamedStdRuntimeWord {
    fn serialize(&self, toplevel: bool) -> SerWord {
        match &self.word {
            RuntimeWord::LiteralVal(ref lit) => SerWord::LiteralVal(*lit),
            RuntimeWord::Verb(_t) => SerWord::Verb(self.name.clone()),
            RuntimeWord::VerbSeq(t) => {
                if toplevel {
                    let mut out = Vec::new();
                    for c in t.inner.iter() {
                        // Also, redundant?
                        out.push(NamedStdRuntimeWord { name: c.name.clone(), word: c.word.clone() }.serialize(false));
                    }
                    SerWord::VerbSeq(out)
                } else {
                    SerWord::Verb(self.name.clone())
                }
            }
            RuntimeWord::UncondRelativeJump { offset } => SerWord::UncondRelativeJump { offset: *offset },
            RuntimeWord::CondRelativeJump { offset, jump_on } => SerWord::CondRelativeJump { offset: *offset, jump_on: *jump_on },
            // Word::LiteralVal(lit) => SerWord::LiteralVal(*lit),
            // Word::Builtin{ name, .. } => SerWord::Builtin { name },
            // Word::Compiled { name, data: comp } => {
            //     if toplevel {
            //         let mut out = Vec::new();
            //         for c in comp.iter() {
            //             out.push(c.serialize(false));
            //         }
            //         SerWord::CompiledDefn(out)
            //     } else {
            //         SerWord::CompiledRef(name.clone())
            //     }
            // },
            // Word::UncondRelativeJump { offset } => SerWord::UncondRelativeJump { offset: *offset },
            // Word::CondRelativeJump { offset, jump_on } => SerWord::CondRelativeJump { offset: *offset, jump_on: *jump_on },
        }
    }
}

pub struct Dict {
    pub(crate) data: BTreeMap<String, NamedStdRuntimeWord>,
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

pub struct Context {
    pub rt: StdRuntime,
    dict: Dict,
}

impl Context {
    //     pub fn serialize(&self) -> SerDict {
    //         let mut dict = self.dict.serialize();

    //         let mut bi_set = BTreeSet::new();

    //         for (_key, var) in dict.data.iter() {
    //             if let SerWord::CompiledDefn(cwrd) = var {
    //                 for w in cwrd {
    //                     if let SerWord::Builtin { name } = w {
    //                         bi_set.insert(*name);
    //                     }
    //                 }
    //             }
    //         }

    //         dict.data.retain(|_k, v| {
    //             match v {
    //                 SerWord::LiteralVal(_) => true,
    //                 SerWord::Builtin { .. } => false,
    //                 SerWord::CompiledDefn(_) => true,
    //                 SerWord::CompiledRef(_) => todo!(),
    //                 SerWord::UncondRelativeJump { .. } => true,
    //                 SerWord::CondRelativeJump { .. } => true,
    //             }
    //         });

    //         dict
    //     }

    pub fn step(&mut self) -> Result<StepResult<BuiltinToken>, Error> {
        self.rt.step()
    }

    //     pub fn data_stack(&self) -> &StdVecStack<i32> {
    //         &self.data_stk
    //     }

    //     pub fn return_stack(&self) -> &StdVecStack<i32> {
    //         &self.ret_stk
    //     }

    //     pub fn flow_stack(&self) -> &StdVecStack<ExecCtx> {
    //         &self.flow_stk
    //     }

    pub fn with_builtins(bi: &[(&'static str, fn(&mut StdRuntime) -> Result<(), Error>)]) -> Self {
        let mut new = Context {
            rt: new_runtime(),
            dict: Dict::new(),
        };

        for (word, func) in bi {
            new.dict.data.insert(
                word.to_string(),
                NamedStdRuntimeWord {
                    word: RuntimeWord::Verb(BuiltinToken::new(*func)),
                    name: word.to_string(),
                },
            );
        }

        new
    }

    pub fn output(&mut self) -> String {
        self.rt.exchange_output()
    }

    pub fn push_exec(&mut self, word: StdRuntimeWord) {
        self.rt.push_exec(word)
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

fn compile(
    ctxt: &mut Context,
    data: &[String],
) -> Result<Vec<Hax>, Error> {
    let mut output: Vec<Hax> = Vec::new();

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

                Hax {
                    word: RuntimeWord::CondRelativeJump {
                        offset,
                        jump_on: false,
                    },
                    name: "CRJ".into(),
                }
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

                Hax {
                    word: RuntimeWord::UncondRelativeJump { offset },
                    name: "UCRJ".into(),
                }
            }
            "then" => {
                then_ct += 1;
                // For now, we only using 'then' as a sentinel value for if/else
                continue;
            }
            "do" => {
                output.push(Hax {
                    word: RuntimeWord::Verb(BuiltinToken::new(
                        afc::builtins::bi_retstk_push,
                    )),
                    name: ">r".into(),
                });
                output.push(Hax {
                    word: RuntimeWord::Verb(BuiltinToken::new(
                        afc::builtins::bi_retstk_push,
                    )),
                    name: ">r".into(),
                });
                do_ct += 1;
                continue;
            }
            "loop" => {
                output.push(Hax {
                    word: RuntimeWord::Verb(BuiltinToken::new(
                        afc::builtins::bi_priv_loop,
                    )),
                    name: "PRIV_LOOP".into(),
                });

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

                Hax {
                    word: RuntimeWord::CondRelativeJump {
                        offset: (-1i32 * offset as i32) - 2,
                        jump_on: false,
                    },
                    name: "CRJ".into(),
                }
            }

            // Now, check for "normal" words, e.g. numeric literals or dictionary words
            other => {
                if let Some(dword) = ctxt.dict.data.get(other).cloned() {
                    Hax {
                        word: dword.word,
                        name: other.to_string(),
                    }
                } else if let Some(num) = parse_num(other) {
                    Hax {
                        word: RuntimeWord::LiteralVal(num),
                        name: format!("LIT({})", num),
                    }
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

            let compiled = Arc::new(compile(ctxt, relevant)?);

            ctxt.dict.data.insert(
                name.clone(),
                NamedStdRuntimeWord {
                    name: name.clone(),
                    word: RuntimeWord::VerbSeq(StdFuncSeq { inner: compiled }),
                }
            );
        }
        _ => {
            // We should interpret this as a line to compile and run
            // (but then discard, because it isn't bound in the dict)
            let temp_compiled = RuntimeWord::VerbSeq(StdFuncSeq { inner: Arc::new(compile(ctxt, &data)?) });
            ctxt.push_exec(temp_compiled);
        }
    }

    Ok(())
}
