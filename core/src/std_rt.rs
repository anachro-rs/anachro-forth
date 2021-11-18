use std::marker::PhantomData;
use std::sync::Arc;

use crate::FuncSeq;
use crate::Runtime;
use crate::RuntimeSeqCtx;
use crate::RuntimeWord;
use crate::{Error, ExecutionStack, Stack};

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

    fn last(&self) -> Result<&Self::Item, Error> {
        self.data.last().ok_or(Error::InternalError) // TODO: Wrong error!
    }
}

impl<T, F> ExecutionStack<T, F> for StdVecStack<RuntimeSeqCtx<T, F>>
where
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    fn push(&mut self, data: RuntimeSeqCtx<T, F>) {
        self.data.push(data)
    }
    fn pop(&mut self) -> Result<RuntimeSeqCtx<T, F>, Error> {
        self.data.pop().ok_or(Error::FlowStackEmpty)
    }
    fn last_mut(&mut self) -> Result<&mut RuntimeSeqCtx<T, F>, Error> {
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
    StdFuncSeq,
    StdVecStack<i32>,
    StdVecStack<RuntimeSeqCtx<BuiltinToken, StdFuncSeq>>,
    String,
>;

#[derive(Clone)]
pub struct Hax {
    pub name: String,
    pub word: RuntimeWord<BuiltinToken, StdFuncSeq>
}

#[derive(Clone)]
pub struct StdFuncSeq {
    pub inner: Arc<Vec<Hax>>,
}

impl FuncSeq<BuiltinToken, Self> for StdFuncSeq {
    fn get(&self, idx: usize) -> Option<RuntimeWord<BuiltinToken, Self>> {
        match self.inner.get(idx) {
            Some(artw) => Some(artw.word.clone()),
            None => None,
        }
    }
}

pub type StdRuntimeWord = RuntimeWord<BuiltinToken, StdFuncSeq>;

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
