use std::marker::PhantomData;

use crate::Runtime;
use crate::RuntimeWord;
use crate::RuntimeSeqCtx;
use crate::FuncSeq;
use crate::{Error, Stack, ExecutionStack};

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
pub struct SeqTok<'a> {
    pub(crate) inner: &'a [RuntimeWord<Toker<'a>, SeqTok<'a>>],
}

impl<'a> FuncSeq<Toker<'a>, SeqTok<'a>> for SeqTok<'a> {
    fn get(&self, idx: usize) -> Option<RuntimeWord<Toker<'a>, SeqTok<'a>>> {
        self.inner.get(idx).map(Clone::clone)
    }
}

#[derive(Clone)]
pub struct Toker<'a> {
    bi: Builtin<'a>,
}

impl<'a> Toker<'a> {
    pub fn new(bi: Builtin<'a>) -> Self {
        Self {
            bi
        }
    }

    pub fn exec(&self, rt: &mut StdRuntime<'a>) -> Result<(), Error> {
        (self.bi)(rt)
    }
}


pub type StdRuntime<'a> = Runtime<
    Toker<'a>,
    StdFuncSeq<'a>,
    StdVecStack<i32>,
    StdVecStack<RuntimeSeqCtx<Toker<'a>, StdFuncSeq<'a>>>,
    String,
>;

#[derive(Clone)]
pub struct StdFuncSeq<'a> {
    pub inner: Vec<RuntimeWord<Toker<'a>, StdFuncSeq<'a>>>,
}

impl<'a> FuncSeq<Toker<'a>, Self> for StdFuncSeq<'a>
where
{
    fn get(&self, idx: usize) -> Option<RuntimeWord<Toker<'a>, Self>> {
        match self.inner.get(idx) {
            Some(artw) => Some(artw.clone()),
            None => None,
        }
    }
}

pub type StdRuntimeWord<'a> = RuntimeWord<
    Toker<'a>,
    StdFuncSeq<'a>,
>;

type Builtin<'a> = fn(
    &mut StdRuntime<'a>,
) -> Result<(), Error>;

pub fn new_runtime<'a>() -> StdRuntime<'a> {
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

pub fn new_funcs<'a>() -> Vec<(&'static str, fn(&mut StdRuntime<'a>) -> Result<(), Error>)> {
    vec![
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

// pub const STD_BUILT_IN_WORDS: &[(&'static str, fn(&mut StdRuntime<'a>) -> Result<(), Error>)] = &[
//     ("emit", crate::builtins::bi_emit),
//     (".", crate::builtins::bi_pop),
//     ("cr", crate::builtins::bi_cr),
//     (">r", crate::builtins::bi_retstk_push),
//     ("r>", crate::builtins::bi_retstk_pop),
//     ("=", crate::builtins::bi_eq),
//     ("<", crate::builtins::bi_lt),
//     (">", crate::builtins::bi_gt),
//     ("dup", crate::builtins::bi_dup),
//     ("+", crate::builtins::bi_add),
//     // TODO: This requires the ability to modify the input stream!
//     //
//     // This is supposed to return the address of the NEXT word in the
//     // input stream
//     //
//     // ("'", bi_tick)

//     // ( @ is the "load operator )  ok
//     // ( ! is the "store operator" )  ok

//     // Constants store a VALUE in the dict, which will be pushed on the stack
//     //
//     // I *think*:
//     //
//     // 5 CONSTANT X
//     //
//     // is equivalent to:
//     //
//     // : X 5 ;

//     // Variables store the value, and put the ADDRESS on the stack when invoked
//     //
//     // I *think*:
//     //
//     // 0 VARIABLE ZERO
//     //
//     // is equvalent to:
//     //
//     // ).unwrap().unwrap()

//     // Debug
//     // ("serdump", crate::builtins::bi_serdump),
//     // ("coredump", crate::builtins::bi_coredump),
// ];
