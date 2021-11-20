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

impl<BuiltinTok, SeqTok, const N: usize> ExecutionStack<BuiltinTok, SeqTok> for HVecStack<RuntimeWord<BuiltinTok, SeqTok>, N>
where
    SeqTok: Clone,
    BuiltinTok: Clone,
{
    fn push(&mut self, data: RuntimeWord<BuiltinTok, SeqTok>) {
        // TODO
        self.data.push(data).map_err(drop).unwrap()
    }
    fn pop(&mut self) -> Result<RuntimeWord<BuiltinTok, SeqTok>, Error> {
        self.data.pop().ok_or(Error::FlowStackEmpty)
    }
    fn last_mut(&mut self) -> Result<&mut RuntimeWord<BuiltinTok, SeqTok>, Error> {
        self.data.last_mut().ok_or(Error::FlowStackEmpty)
    }
}

#[derive(Clone)]
pub struct BuiltinToken<const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> {
    bi: Builtin<DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
}

impl<const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize>
    BuiltinToken<DATA_SZ, FLOW_SZ, OUTBUF_SZ>
{
    pub fn new(bi: Builtin<DATA_SZ, FLOW_SZ, OUTBUF_SZ>) -> Self {
        Self { bi }
    }

    pub fn exec(
        &self,
        rt: &mut NoStdRuntime<DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
    ) -> Result<(), Error> {
        (self.bi)(rt)
    }
}

pub type NoStdRuntime<const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> =
    Runtime<
        BuiltinToken<DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
        usize,
        HVecStack<i32, DATA_SZ>,
        HVecStack<
            RuntimeWord<
                BuiltinToken<DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
                usize,
            >,
            FLOW_SZ,
        >,
        String<OUTBUF_SZ>,
    >;


pub type NoStdRuntimeWord<const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> = RuntimeWord<BuiltinToken<DATA_SZ, FLOW_SZ, OUTBUF_SZ>, usize>;

pub type Builtin<const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize> =
    fn(&mut NoStdRuntime<DATA_SZ, FLOW_SZ, OUTBUF_SZ>) -> Result<(), Error>;

pub fn new_runtime<const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize>(
) -> NoStdRuntime<DATA_SZ, FLOW_SZ, OUTBUF_SZ> {
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

pub fn std_builtins<const DATA_SZ: usize, const FLOW_SZ: usize, const OUTBUF_SZ: usize>(
) -> &'static [(
    &'static str,
    fn(&mut NoStdRuntime<DATA_SZ, FLOW_SZ, OUTBUF_SZ>) -> Result<(), Error>,
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
