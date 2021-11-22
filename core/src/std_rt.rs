use std::convert::TryInto;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::Runtime;
use crate::RuntimeWord;
use crate::{Error, ExecutionStack, Stack};
use crate::ser_de::SerWord;

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

impl<T> StdVecStack<T> {
    pub fn data(&self) -> &[T] {
        &self.data
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

    fn last(&self) -> Result<&Self::Item, Error> {
        self.data.last().ok_or(Error::InternalError) // TODO: Wrong error!
    }
}

impl<T, F> ExecutionStack<T, F> for StdVecStack<RuntimeWord<T, F>>
where
    F: Clone,
    T: Clone,
{
    fn push(&mut self, data: RuntimeWord<T, F>) {
        self.data.push(data)
    }
    fn pop(&mut self) -> Result<RuntimeWord<T, F>, Error> {
        self.data.pop().ok_or(Error::FlowStackEmpty)
    }
    fn last_mut(&mut self) -> Result<&mut RuntimeWord<T, F>, Error> {
        self.data.last_mut().ok_or(Error::FlowStackEmpty)
    }
}

#[derive(Clone)]
pub struct BuiltinToken {
    bi: Builtin,
}

impl BuiltinToken {
    pub fn new(bi: Builtin) -> Self {
        Self { bi }
    }

    pub fn exec(&self, rt: &mut StdRuntime) -> Result<(), Error> {
        (self.bi)(rt)
    }
}

pub type StdRuntime = Runtime<
    BuiltinToken,
    String,
    StdVecStack<i32>,
    StdVecStack<RuntimeWord<BuiltinToken, String>>,
    String,
>;

#[derive(Clone)]
pub struct NamedStdRuntimeWord {
    pub name: String,
    pub word: RuntimeWord<BuiltinToken, String>,
}

#[derive(Clone)]
pub struct StdFuncSeq {
    pub inner: Arc<Vec<NamedStdRuntimeWord>>,
}


pub type StdRuntimeWord = RuntimeWord<BuiltinToken, String>;

type Builtin = fn(&mut StdRuntime) -> Result<(), Error>;

pub fn new_runtime() -> StdRuntime {
    // These are the only data structures required, and Runtime is generic over the
    // stacks, so I could easily use heapless::Vec as a backing structure as well
    let ds = StdVecStack::new(Error::DataStackEmpty);
    let rs = StdVecStack::new(Error::RetStackEmpty);
    let fs = StdVecStack::new(Error::FlowStackEmpty);

    // This is a generic Runtime type, I'll likely define two versions:
    // One with std-ability (for the host), and one no-std one, so users
    // wont have to deal with all the generic shenanigans
    Runtime {
        data_stk: ds,
        ret_stk: rs,
        flow_stk: fs,
        _pd_ty_t_f: PhantomData,
        cur_output: String::new(),
    }
}

pub fn std_builtins() -> &'static [(&'static str, fn(&mut StdRuntime) -> Result<(), Error>)] {
    &[
        ("emit", crate::builtins::bi_emit),
        (".", crate::builtins::bi_pop),
        ("cr", crate::builtins::bi_cr),
        (">r", crate::builtins::bi_retstk_push),
        ("r>", crate::builtins::bi_retstk_pop),
        ("=", crate::builtins::bi_eq),
        ("<", crate::builtins::bi_lt),
        (">", crate::builtins::bi_gt),
        ("dup", crate::builtins::bi_dup),
        ("+", crate::builtins::bi_add),
    ]
}


pub struct SerContext {
    pub bis: Vec<String>,
    pub seqs: Vec<String>,
}

impl SerContext {
    pub fn new() -> Self {
        Self {
            bis: Vec::new(),
            seqs: Vec::new(),
        }
    }

    pub fn encode_rtw(&mut self, word: &NamedStdRuntimeWord) -> SerWord {
        match &word.word {
            RuntimeWord::LiteralVal(lit) => SerWord::LiteralVal(*lit),
            RuntimeWord::Verb(_) => {
                let idx = self.intern_bis(&word.name);
                SerWord::Verb(idx)
            },
            RuntimeWord::VerbSeq(seq) => {
                let idx = self.intern_seq(&seq.tok);
                SerWord::VerbSeq(idx)
            },
            RuntimeWord::UncondRelativeJump { offset } => SerWord::UncondRelativeJump { offset: *offset },
            RuntimeWord::CondRelativeJump { offset, jump_on } => SerWord::CondRelativeJump { offset: *offset, jump_on: *jump_on },
        }
    }

    pub fn intern_bis(&mut self, word: &str) -> u16 {
        if let Some(pos) = self.bis.iter().position(|w| word == w) {
            pos
        } else {
            self.bis.push(word.to_string());
            self.bis.len() - 1
        }.try_into().unwrap()
    }

    pub fn intern_seq(&mut self, word: &str) -> u16 {
        if let Some(pos) = self.seqs.iter().position(|w| word == w) {
            pos
        } else {
            self.seqs.push(word.to_string());
            self.seqs.len() - 1
        }.try_into().unwrap()
    }
}

// TODO: Make a method of NamedStdRuntimeWord
pub fn ser_srw(ctxt: &mut SerContext, name: &str, words: &StdFuncSeq) -> Vec<SerWord> {
    let mut out = vec![];

    for word in words.inner.iter() {
        let new = ctxt.encode_rtw(word);
        out.push(new);
    }

    // Ensure that the currently encoded word makes it into
    // the list of interned words
    let _ = ctxt.intern_seq(name);

    out
}
