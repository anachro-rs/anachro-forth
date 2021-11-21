use std::sync::Arc;
use std::collections::{BTreeMap, BTreeSet};
use serde::Serialize;

use afc::std_rt::{BuiltinToken, NamedStdRuntimeWord, StdFuncSeq, StdRuntimeWord, StdVecStack, new_runtime, std_builtins};
use afc::{RuntimeWord, StepResult, VerbSeqInner};
use anachro_forth_core as afc;

use afc::{std_rt::StdRuntime, Error};


#[derive(Debug, Serialize, Clone)]
pub enum SerWord {
    LiteralVal(i32),
    Verb(String),
    VerbSeq(String),
    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

// TODO: Make a method of NamedStdRuntimeWord
fn ser_srw(words: &StdFuncSeq) -> Vec<SerWord> {
    let mut out = vec![];

    for word in words.inner.iter() {
        let new = match &word.word {
            RuntimeWord::LiteralVal(lit) => SerWord::LiteralVal(*lit),
            RuntimeWord::Verb(_) => SerWord::Verb(word.name.clone()),
            RuntimeWord::VerbSeq(seq) => {
                SerWord::VerbSeq(seq.tok.clone())
            },
            RuntimeWord::UncondRelativeJump { offset } => SerWord::UncondRelativeJump { offset: *offset },
            RuntimeWord::CondRelativeJump { offset, jump_on } => SerWord::CondRelativeJump { offset: *offset, jump_on: *jump_on },
        };
        out.push(new);
    }

    out
}


pub struct Dict {
    pub bis: BTreeMap<String, BuiltinToken>,
    pub data: BTreeMap<String, StdFuncSeq>,
    pub(crate) shame_idx: usize,
}

#[derive(Debug, Serialize)]
pub struct SerDict {
    data: Vec<(String, Vec<SerWord>)>,
    bis: Vec<String>,
}

impl Dict {
    pub fn new() -> Self {
        Self {
            bis: BTreeMap::new(),
            data: BTreeMap::new(),
            shame_idx: 0,
        }
    }

    pub fn serialize(&self) -> SerDict {
        let mut out: BTreeMap<String, Vec<SerWord>> = BTreeMap::new();

        for (word, val) in self.data.iter() {
            out.insert(word.clone(), ser_srw(val));
            // println!("UHOH: {}", word);
        }

        println!("{:?}", out);

        let mut all_verbs: BTreeSet<String> = BTreeSet::new();
        let mut used_verbs: BTreeSet<String> = BTreeSet::new();

        for (name, _) in std_builtins() {
            all_verbs.insert(name.to_string());
        }

        let mut vout: BTreeSet<String> = BTreeSet::new();
        let mut data: Vec<(String, Vec<SerWord>)> = vec![];

        // This is to ensure that items are serialized in the necessary order,
        // e.g. if `mstar` calls `star`, `star` is guaranteed to be serialized
        // first in order.

        // This MAY not be necessary, depending on how I do the deserialization,
        // with two passes, it's likely it doesn't matter
        while !out.is_empty() {
            let nowt = out.clone();

            'seqs: for (k, v) in nowt.iter() {
                'seqelem: for val in v {
                    match val {
                        SerWord::LiteralVal(_) => continue 'seqelem,
                        SerWord::Verb(sw) => {
                            if all_verbs.contains(sw) {
                                used_verbs.insert(sw.to_string());
                            } else {
                                panic!()
                            }
                        }
                        SerWord::VerbSeq(vs) => {
                            if vout.contains(vs) {
                                continue 'seqelem;
                            } else {
                                continue 'seqs;
                            }
                        },
                        SerWord::UncondRelativeJump { .. } => continue 'seqelem,
                        SerWord::CondRelativeJump { .. } => continue 'seqelem,
                    }
                }

                out.remove(k);
                vout.insert(k.to_string());
                data.push((k.to_string(), v.clone()));
            }
        }

        SerDict { data, bis: used_verbs.iter().cloned().collect() }
    }
}

pub struct Context {
    pub rt: StdRuntime,
    pub dict: Dict,
}

impl Context {
    pub fn serialize(&self) -> SerDict {
        self.dict.serialize()
    }

    pub fn step(&mut self) -> Result<StepResult<BuiltinToken, String>, Error> {
        self.rt.step()
    }

    pub fn data_stack(&self) -> &StdVecStack<i32> {
        &self.rt.data_stk
    }

    pub fn return_stack(&self) -> &StdVecStack<i32> {
        &self.rt.ret_stk
    }

    pub fn flow_stack(&self) -> &StdVecStack<RuntimeWord<BuiltinToken, String>> {
        &self.rt.flow_stk
    }

    pub fn with_builtins(bi: &[(&'static str, fn(&mut StdRuntime) -> Result<(), Error>)]) -> Self {
        let mut new = Context {
            rt: new_runtime(),
            dict: Dict::new(),
        };

        for (word, func) in bi {
            new.dict.bis.insert(
                word.to_string(),
                BuiltinToken::new(*func),
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
) -> Result<Vec<NamedStdRuntimeWord>, Error> {
    let mut output: Vec<NamedStdRuntimeWord> = Vec::new();

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

                NamedStdRuntimeWord {
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

                NamedStdRuntimeWord {
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
                output.push(NamedStdRuntimeWord {
                    word: RuntimeWord::Verb(BuiltinToken::new(
                        afc::builtins::bi_retstk_push,
                    )),
                    name: ">r".into(),
                });
                output.push(NamedStdRuntimeWord {
                    word: RuntimeWord::Verb(BuiltinToken::new(
                        afc::builtins::bi_retstk_push,
                    )),
                    name: ">r".into(),
                });
                do_ct += 1;
                continue;
            }
            "loop" => {
                output.push(NamedStdRuntimeWord {
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

                NamedStdRuntimeWord {
                    word: RuntimeWord::CondRelativeJump {
                        offset: (-1i32 * offset as i32) - 2,
                        jump_on: false,
                    },
                    name: "CRJ".into(),
                }
            }

            // Now, check for "normal" words, e.g. numeric literals or dictionary words
            other => {
                if let Some(bi) = ctxt.dict.bis.get(other).cloned() {
                    NamedStdRuntimeWord {
                        name: other.to_string(),
                        word: RuntimeWord::Verb(bi.clone()),
                    }
                } else if ctxt.dict.data.contains_key(other) {
                    NamedStdRuntimeWord {
                        word: RuntimeWord::VerbSeq(VerbSeqInner::from_word(other.to_string())),
                        name: other.to_string(),
                    }
                } else if let Some(num) = parse_num(other) {
                    NamedStdRuntimeWord {
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
                name,
                StdFuncSeq { inner: compiled },
            );
        }
        _ => {
            // We should interpret this as a line to compile and run
            // (but then discard, because it isn't bound in the dict)
            // let temp_compiled = RuntimeWord::VerbSeq(StdFuncSeq { inner:  });
            if !data.is_empty() {
                let name = format!("__{}", ctxt.dict.shame_idx);
                let comp = compile(ctxt, &data)?;
                ctxt.dict.data.insert(
                    name.clone(),
                    StdFuncSeq { inner: Arc::new(comp) },
                );
                ctxt.dict.shame_idx += 1;
                let temp_compiled = RuntimeWord::VerbSeq(VerbSeqInner::from_word(name));
                ctxt.push_exec(temp_compiled);
            }
        }
    }

    Ok(())
}
