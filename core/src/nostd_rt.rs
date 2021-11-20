use core::marker::PhantomData;

use crate::Runtime;
use crate::RuntimeWord;
use crate::{Error, ExecutionStack, Stack};

use heapless::{String, Vec};

#[derive(Debug)]
pub struct HVecStack<T, const N: usize> {
    data: Vec<T, N>,
    err: Error,
}

impl<T, const N: usize> HVecStack<T, N> {
    pub fn new(err: Error) -> Self {
        HVecStack {
            data: Vec::new(),
            err,
        }
    }
}

impl<T, const N: usize> Stack for HVecStack<T, N> {
    type Item = T;

    fn push(&mut self, data: T) {
        self.data.push(data).map_err(drop).unwrap();
    }

    fn pop(&mut self) -> Result<T, Error> {
        self.data.pop().ok_or(Error::DataStackUnderflow)
    }

    fn last(&self) -> Result<&Self::Item, Error> {
        self.data.last().ok_or(Error::InternalError) // TODO: Wrong error!
    }
}

impl<T, FuncTok, const N: usize> ExecutionStack<T, FuncTok> for HVecStack<RuntimeWord<T, FuncTok>, N>
where
    FuncTok: Clone,
    T: Clone,
{
    fn push(&mut self, data: RuntimeWord<T, FuncTok>) {
        // TODO
        self.data.push(data).map_err(drop).unwrap()
    }
    fn pop(&mut self) -> Result<RuntimeWord<T, FuncTok>, Error> {
        self.data.pop().ok_or(Error::FlowStackEmpty)
    }
    fn last_mut(&mut self) -> Result<&mut RuntimeWord<T, FuncTok>, Error> {
        self.data.last_mut().ok_or(Error::FlowStackEmpty)
    }
}

#[derive(Clone)]
pub struct BuiltinToken<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> {
    bi: Builtin<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
}

impl<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize>
    BuiltinToken<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>
{
    pub fn new(bi: Builtin<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>) -> Self {
        Self { bi }
    }

    pub fn exec(
        &self,
        rt: &mut NoStdRuntime<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
    ) -> Result<(), Error> {
        (self.bi)(rt)
    }
}

pub type NoStdRuntime<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> =
    Runtime<
        BuiltinToken<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
        NoStdFuncSeq<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
        HVecStack<i32, DATA_SZ>,
        HVecStack<
            RuntimeWord<
                BuiltinToken<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
                NoStdFuncSeq<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
            >,
            FLOW_SZ,
        >,
        String<OUTBUF_SZ>,
    >;

#[derive(Clone)]
pub struct NoStdFuncSeq<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> {
    pub inner: &'a [RuntimeWord<
        BuiltinToken<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
        NoStdFuncSeq<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
    >],
}

// impl<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize>
//     FuncSeq<BuiltinToken<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>, Self>
//     for NoStdFuncSeq<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>
// {
//     // TODO(AJM): This impl is holding me back. It requires that every
//     // FuncSeq can produce the next word on-demand, which would be hard
//     // to do without holding the entire sequence, which may recurse.
//     //
//     // I wonder if I could get tricky, and have a builtin which loads a
//     // sequence lazily? Or something that would allow me to smuggle this
//     // out. Otherwise, the only thing I can think of is to have a dict
//     // crate, that can summon builtins/sequences on-demand, and pass that
//     // in on every call to `step`...
//     //
//     // I mean, I guess the runtime could own the dictionary again, it'd just
//     // introduce two more generic parameters, one for the max sequence length,
//     // and one for the number of sequences that could be held...
//     fn get(
//         &self,
//         idx: usize,
//     ) -> Option<RuntimeWord<BuiltinToken<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>, Self>> {
//         match self.inner.get(idx) {
//             Some(artw) => Some(artw.clone()),
//             None => None,
//         }
//     }
// }

pub type NoStdRuntimeWord<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> = RuntimeWord<BuiltinToken<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>, NoStdFuncSeq<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>>;

type Builtin<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> =
    fn(&mut NoStdRuntime<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>) -> Result<(), Error>;

pub fn new_runtime<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize>(
) -> NoStdRuntime<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ> {
    // These are the only data structures required, and Runtime is generic over the
    // stacks, so I could easily use heapless::Vec as a backing structure as well
    let ds = HVecStack::new(Error::DataStackEmpty);
    let rs = HVecStack::new(Error::RetStackEmpty);
    let fs = HVecStack::new(Error::FlowStackEmpty);

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

pub fn std_builtins<'a, const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize>(
) -> &'static [(
    &'static str,
    fn(&mut NoStdRuntime<'a, DATA_SZ, FLOW_SZ, OUTBUF_SZ>) -> Result<(), Error>,
)] {
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
