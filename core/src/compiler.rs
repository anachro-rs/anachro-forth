use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    ser_de::{SerDict, SerWord},
    std_rt::{
        new_runtime, ser_srw, BuiltinToken, NamedStdRuntimeWord, SerContext, StdFuncSeq,
        StdRuntime, StdRuntimeWord, StdVecStack,
    },
    Error, RuntimeWord, StepResult, VerbSeqInner,
};

pub struct Dict {
    pub bis: BTreeMap<String, BuiltinToken>,
    pub data: BTreeMap<String, StdFuncSeq>,
    pub(crate) shame_idx: usize,
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
        let mut data_map: Vec<String> = Vec::new();
        let mut ctxt = SerContext::new();

        for (word, val) in self.data.iter() {
            out.insert(word.to_string(), ser_srw(&mut ctxt, &word, val));
        }

        let mut data = Vec::new();
        for word in ctxt.seqs {
            data.push(out.get(&word).unwrap().clone());
            data_map.push(word.clone());
        }

        SerDict {
            data,
            data_map: Some(data_map),
            bis: ctxt.bis,
        }
    }
}

pub struct Context {
    pub rt: StdRuntime,
    pub dict: Dict,
}

impl Context {
    pub fn load_ser_dict(&mut self, data: &SerDict) {
        let data_map = if let Some(dm) = data.data_map.as_ref() {
            dm.clone()
        } else {
            eprintln!("Error: dict has no name map! Refusing to load.");
            return;
        };

        if !data.bis.iter().all(|bi| self.dict.bis.contains_key(bi)) {
            eprintln!("Missing builtins! Refusing to load.");
            return;
        }

        if data_map.len() != data.data.len() {
            eprintln!("Data map size mismatch! Refusing to load.");
            return;
        }

        for (name, word) in data_map.iter().zip(data.data.iter()) {
            let cword = word.iter().map(|x| {
                match x {
                    SerWord::LiteralVal(v) => NamedStdRuntimeWord { name: format!("LIT({})", v), word: RuntimeWord::LiteralVal(*v) },
                    SerWord::Verb(i) => {
                        let txt = data.bis.get(*i as usize).unwrap();
                        NamedStdRuntimeWord {
                            name: txt.clone(),
                            word: RuntimeWord::Verb(self.dict.bis.get(txt).unwrap().clone()),
                        }
                    }
                    SerWord::VerbSeq(i) => {
                        let txt = data_map.get(*i as usize).unwrap();
                        NamedStdRuntimeWord {
                            name: txt.clone(),
                            word: RuntimeWord::VerbSeq(VerbSeqInner::from_word(txt.to_string())),
                        }
                    },
                    SerWord::UncondRelativeJump { offset } => NamedStdRuntimeWord {
                        name: format!("UCRJ({})", offset),
                        word: RuntimeWord::UncondRelativeJump { offset: *offset }
                    },
                    SerWord::CondRelativeJump { offset, jump_on } => NamedStdRuntimeWord {
                        name: format!("CRJ({})", offset),
                        word: RuntimeWord::CondRelativeJump { offset: *offset, jump_on: *jump_on }
                    },
                }
            }).collect::<Vec<_>>();

            self.dict.data.insert(name.clone(), StdFuncSeq { inner: Arc::new(cword) });
        }
    }

    fn compile(&mut self, data: &[String]) -> Result<Vec<NamedStdRuntimeWord>, Error> {
        let mut vd_data: VecDeque<String> = data.iter().map(String::as_str).map(str::to_lowercase).collect();

        let munched = muncher(&mut vd_data);
        assert!(vd_data.is_empty());

        let conv: Vec<NamedStdRuntimeWord> = munched.into_iter().map(|m| m.to_named_rt_words(&mut self.dict)).flatten().collect();

        Ok(conv)
    }

    pub fn evaluate(&mut self, data: Vec<String>) -> Result<(), Error> {
        match (data.first(), data.last()) {
            (Some(f), Some(l)) if f == ":" && l == ";" => {
                // Must have ":", "$NAME", "$SOMETHING+", ";"
                assert!(data.len() >= 3);

                let name = data[1].to_lowercase();

                // TODO: Doesn't handle "empty" definitions
                let relevant = &data[2..][..data.len() - 3];

                // let compiled = Arc::new(self.compile(relevant)?);
                let compiled = Arc::new(self.compile(relevant).unwrap());

                self.dict.data.insert(name, StdFuncSeq { inner: compiled });
            }
            _ => {
                // We should interpret this as a line to compile and run
                // (but then discard, because it isn't bound in the dict)
                // let temp_compiled = RuntimeWord::VerbSeq(StdFuncSeq { inner:  });
                if !data.is_empty() {
                    let name = format!("__{}", self.dict.shame_idx);
                    // let comp = self.compile(&data)?;
                    let comp = self.compile(&data).unwrap();
                    self.dict.data.insert(
                        name.clone(),
                        StdFuncSeq {
                            inner: Arc::new(comp),
                        },
                    );
                    self.dict.shame_idx += 1;
                    let temp_compiled = RuntimeWord::VerbSeq(VerbSeqInner::from_word(name));
                    self.push_exec(temp_compiled);
                }
            }
        }

        Ok(())
    }



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
            new.dict
                .bis
                .insert(word.to_string(), BuiltinToken::new(*func));
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


/// This struct represents a "chunk" of the AST
#[derive(Debug)]
enum Chunk {
    IfThen {
        if_body: Vec<Chunk>,
    },
    IfElseThen {
        if_body: Vec<Chunk>,
        else_body: Vec<Chunk>,
    },
    DoLoop {
        do_body: Vec<Chunk>,
    },
    Token(String),
}

impl Chunk {
    /// Convert a chunk of AST words into a vec of `NamedStdRuntimeWord`s
    fn to_named_rt_words(self, dict: &mut Dict) -> Vec<NamedStdRuntimeWord> {
        let mut ret = vec![];

        match self {
            Chunk::IfThen { if_body } => {
                // First, convert the body into a sequence
                let mut conv: VecDeque<NamedStdRuntimeWord> = if_body
                    .into_iter()
                    .map(|m| m.to_named_rt_words(dict))
                    .flatten()
                    .collect();

                conv.push_front(NamedStdRuntimeWord {
                    name: "CRJ".into(),
                    word: RuntimeWord::CondRelativeJump { offset: conv.len() as i32, jump_on: false },
                });

                let conv: Vec<NamedStdRuntimeWord> = conv.into_iter().collect();
                ret.extend(conv);
            },
            Chunk::IfElseThen { if_body, else_body } => {
                let mut if_conv: VecDeque<NamedStdRuntimeWord> = if_body
                    .into_iter()
                    .map(|m| m.to_named_rt_words(dict))
                    .flatten()
                    .collect();

                let else_conv: Vec<NamedStdRuntimeWord> = else_body
                    .into_iter()
                    .map(|m| m.to_named_rt_words(dict))
                    .flatten()
                    .collect();

                if_conv.push_back(NamedStdRuntimeWord {
                    name: "UCRJ".into(),
                    word: RuntimeWord::UncondRelativeJump { offset: else_conv.len() as i32 },
                });

                if_conv.push_front(NamedStdRuntimeWord {
                    name: "CRJ".into(),
                    word: RuntimeWord::CondRelativeJump { offset: if_conv.len() as i32, jump_on: false },
                });

                let conv: Vec<NamedStdRuntimeWord> = if_conv.into_iter().chain(else_conv.into_iter()).collect();
                ret.extend(conv);
            },
            Chunk::DoLoop { do_body } => {
                // First, convert the body into a sequence
                let mut conv: VecDeque<NamedStdRuntimeWord> = do_body
                    .into_iter()
                    .map(|m| m.to_named_rt_words(dict))
                    .flatten()
                    .collect();

                conv.push_back(NamedStdRuntimeWord {
                    word: RuntimeWord::Verb(BuiltinToken::new(crate::builtins::bi_priv_loop)),
                    name: "PRIV_LOOP".into(),
                });

                let len = conv.len();

                conv.push_front(NamedStdRuntimeWord {
                    word: RuntimeWord::Verb(BuiltinToken::new(crate::builtins::bi_retstk_push)),
                    name: ">r".into(),
                });
                conv.push_front(NamedStdRuntimeWord {
                    word: RuntimeWord::Verb(BuiltinToken::new(crate::builtins::bi_retstk_push)),
                    name: ">r".into(),
                });

                // The Minus One here accounts for the addition of the CRJ. We should not loop back to
                // the double `>r`s, as those only happen once at the top of the loop.
                conv.push_back(NamedStdRuntimeWord {
                    word: RuntimeWord::CondRelativeJump { offset: -1 * len as i32 - 1, jump_on: false },
                    name: "CRJ".into(),
                });

                let conv: Vec<NamedStdRuntimeWord> = conv.into_iter().collect();
                ret.extend(conv);
            },
            Chunk::Token(tok) => {
                ret.push(if let Some(bi) = dict.bis.get(&tok).cloned() {
                    NamedStdRuntimeWord {
                        name: tok,
                        word: RuntimeWord::Verb(bi.clone()),
                    }
                } else if dict.data.contains_key(&tok) {
                    NamedStdRuntimeWord {
                        word: RuntimeWord::VerbSeq(VerbSeqInner::from_word(tok.clone())),
                        name: tok,
                    }
                } else if let Some(num) = parse_num(&tok) {
                    NamedStdRuntimeWord {
                        word: RuntimeWord::LiteralVal(num),
                        name: format!("LIT({})", num),
                    }
                } else {
                    panic!()
                    // return Err(Error::InternalError);
                });
            },
        }

        ret
    }
}

use std::collections::VecDeque;

fn muncher(data: &mut VecDeque<String>) -> Vec<Chunk> {
    let mut chunks = vec![];
    loop {
        let next = if let Some(t) = data.pop_front() {
            t
        } else {
            break;
        };

        match next.as_str() {
            "do" => {
                chunks.push(munch_do(data));
            }
            "if" => {
                chunks.push(munch_if(data));
            }
            _ => chunks.push(Chunk::Token(next)),
        }
    }

    chunks
}

fn munch_do(data: &mut VecDeque<String>) -> Chunk {
    let mut chunks = vec![];
    loop {
        let next = if let Some(t) = data.pop_front() {
            t
        } else {
            break;
        };

        match next.as_str() {
            "do" => {
                chunks.push(munch_do(data));
            }
            "if" => {
                chunks.push(munch_if(data));
            }
            "loop" => {
                return Chunk::DoLoop {
                    do_body: chunks,
                }
            }
            _ => chunks.push(Chunk::Token(next)),
        }
    }

    // We... shouldn't get here. This means we never found our "loop" after the "do"
    todo!()
}

fn munch_if(data: &mut VecDeque<String>) -> Chunk {
    let mut chunks = vec![];
    loop {
        let next = if let Some(t) = data.pop_front() {
            t
        } else {
            break;
        };

        match next.as_str() {
            "do" => {
                chunks.push(munch_do(data));
            }
            "if" => {
                chunks.push(munch_if(data));
            }
            "then" => {
                return Chunk::IfThen {
                    if_body: chunks,
                }
            }
            "else" => {
                return munch_else(data, chunks);
            }
            _ => chunks.push(Chunk::Token(next)),
        }
    }

    // We... shouldn't get here. This means we never found our "then"/"else" after the "if"
    todo!()
}

fn munch_else(data: &mut VecDeque<String>, if_body: Vec<Chunk>) -> Chunk {
    let mut chunks = vec![];
    loop {
        let next = if let Some(t) = data.pop_front() {
            t
        } else {
            break;
        };

        match next.as_str() {
            "do" => {
                chunks.push(munch_do(data));
            }
            "if" => {
                chunks.push(munch_if(data));
            }
            "then" => {
                return Chunk::IfElseThen {
                    if_body,
                    else_body: chunks,
                }
            }
            _ => chunks.push(Chunk::Token(next)),
        }
    }

    // We... shouldn't get here. This means we never found our "then" after the "else"
    todo!()
}
