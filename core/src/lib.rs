#![cfg_attr(not(any(test, feature = "std")), no_std)]

use core::{fmt::Write, marker::PhantomData};

pub mod builtins;

#[cfg(any(test, feature = "std"))]
pub mod std_rt;

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

    /// Something has gone *terribly* wrong
    InternalError,
}

impl From<core::fmt::Error> for Error {
    fn from(_other: core::fmt::Error) -> Self {
        Self::OutputFormat
    }
}

pub trait FuncSeq<T, F>
where
    F: FuncSeq<T, F>,
    T: Clone,
    F: Clone,
{
    fn get(&self, _idx: usize) -> Option<RuntimeWord<T, F>>;
}

#[derive(Debug, Clone)]
pub enum RuntimeWord<T, F>
where
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    LiteralVal(i32),

    // TODO: Blend these somehow?
    Verb(T),
    VerbSeq(F),

    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

pub struct RuntimeSeqCtx<T, F>
where
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    pub idx: usize,
    pub word: RuntimeWord<T, F>,
}

pub struct Runtime<T, F, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<T, F>,
    F: FuncSeq<T, F> + Clone,
    T: Clone,
    O: Write,
{
    pub data_stk: Sdata,
    pub ret_stk: Sdata,
    pub flow_stk: Sexec,
    pub _pd_ty_t_f: PhantomData<(T, F)>,
    cur_output: O,
}

impl<Sdata, Sexec, T, F, O> Runtime<T, F, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<T, F>,
    F: FuncSeq<T, F> + Clone,
    T: Clone,
    O: Write,
{
    pub fn step(&mut self) -> Result<StepResult<T>, Error> {
        match self.step_inner() {
            Ok(r) => Ok(r),
            Err(e) => {
                while let Ok(_) = self.flow_stk.pop() {}
                while let Ok(_) = self.data_stk.pop() {}
                while let Ok(_) = self.ret_stk.pop() {}
                Err(e)
            }
        }
    }

    fn step_inner(&mut self) -> Result<StepResult<T>, Error> {
        let ret = 'oloop: loop {
            // TODO: I should set a limit to the max number of loop
            // iterations that are made here! Or maybe go back to
            // yielding at each step
            let cur = match self.flow_stk.last_mut() {
                Ok(frame) => frame,
                Err(_) => return Ok(StepResult::Done),
            };

            let mut jump = None;

            enum WhichToken<T, F>
            where
                F: FuncSeq<T, F> + Clone,
                T: Clone,
            {
                Single(T),
                Ref(RuntimeWord<T, F>),
            }

            let to_push = match cur.word.clone() {
                RuntimeWord::LiteralVal(lit) => {
                    // println!("lit");
                    self.data_stk.push(lit);
                    None
                }
                RuntimeWord::Verb(ft) => {
                    // println!("verb");
                    Some(WhichToken::Single(ft))
                }
                RuntimeWord::VerbSeq(seq) => {
                    // println!("verbseq->{}", cur.idx);
                    // TODO: I should probably check for a difference
                    // between exactly one over-bounds (jump to end of seq),
                    // and overshooting (probably an engine error)
                    let ret = seq.get(cur.idx).map(WhichToken::Ref);
                    // println!("\tgot? {}", ret.is_some());
                    cur.idx += 1;
                    ret
                }
                RuntimeWord::UncondRelativeJump { offset } => {
                    // println!("ucrj");
                    jump = Some(offset);
                    None
                }
                RuntimeWord::CondRelativeJump { offset, jump_on } => {
                    // println!("crj");
                    let topvar = self.data_stk.pop()?;

                    // Truth table:
                    // tv == 0 | jump_on | jump
                    // ========|=========|=======
                    // false   | false   | no
                    // true    | false   | yes
                    // false   | true    | yes
                    // true    | true    | no
                    let do_jump = (topvar == 0) ^ jump_on;

                    // println!("topvar: {}, jump_on: {}", topvar, jump_on);

                    if do_jump {
                        // println!("Jumping!");
                        jump = Some(offset);
                    } else {
                        // println!("Not Jumping!");
                    }
                    None
                }
            };

            match to_push {
                Some(WhichToken::Single(ft)) => {
                    // println!("BREAK");
                    self.flow_stk.pop()?;
                    break 'oloop ft;
                }
                Some(WhichToken::Ref(rf)) => {
                    // println!("FLOWPUSH");
                    self.flow_stk.push(RuntimeSeqCtx { idx: 0, word: rf });
                }
                None => {
                    // println!("FLOWPOP");
                    self.flow_stk.pop()?;
                }
            }

            if let Some(jump) = jump {
                // We just popped off the jump command, so now we are back in
                // the "parent" frame.

                let new_cur = self.flow_stk.last_mut()?;

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
        };

        Ok(StepResult::Working(ret))
    }

    pub fn push_exec(&mut self, word: RuntimeWord<T, F>) {
        self.flow_stk.push(RuntimeSeqCtx { idx: 0, word });
    }
}

impl<Sdata, Sexec, T, F, O> Runtime<T, F, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<T, F>,
    F: FuncSeq<T, F> + Clone,
    T: Clone,
    O: Write + Default,
{
    pub fn exchange_output(&mut self) -> O {
        let mut new = O::default();
        core::mem::swap(&mut new, &mut self.cur_output);
        new
    }
}

pub trait Stack {
    type Item;

    fn push(&mut self, data: Self::Item);
    fn pop(&mut self) -> Result<Self::Item, Error>;

    // Needed for builtins
    fn last(&self) -> Result<&Self::Item, Error>;
}

pub trait ExecutionStack<T, F>
where
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    fn push(&mut self, data: RuntimeSeqCtx<T, F>);
    fn pop(&mut self) -> Result<RuntimeSeqCtx<T, F>, Error>;
    fn last_mut(&mut self) -> Result<&mut RuntimeSeqCtx<T, F>, Error>;
}

pub enum StepResult<T> {
    Done,
    Working(T),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::std_rt::*;
    use std::sync::Arc;

    #[test]
    fn foo() {
        let mut x = new_runtime();

        // Manually craft a word, roughly:
        // : star 42 emit ;
        let pre_seq = StdFuncSeq {
            inner: Arc::new(vec![
                Hax {
                    word: RuntimeWord::LiteralVal(42),
                    name: "42".into(),
                },
                Hax {
                    word: RuntimeWord::Verb(BuiltinToken::new(builtins::bi_emit)),
                    name: "emit".into(),
                }
            ]),
        };

        // Manually craft another word, roughly:
        // : mstar star -1 if star star then ;
        let seq = StdFuncSeq {
            inner: Arc::new(vec![
                Hax { word: RuntimeWord::VerbSeq(pre_seq.clone()), name: "star".into() },
                Hax { word: RuntimeWord::LiteralVal(-1), name: "-1".into() },
                Hax { word: RuntimeWord::CondRelativeJump {
                    offset: 2,
                    jump_on: false,
                }, name: "UCRJ".into() },
                Hax { word: RuntimeWord::VerbSeq(pre_seq.clone()), name: "star".into() },
                Hax { word: RuntimeWord::VerbSeq(pre_seq.clone()), name: "star".into() },
            ]),
        };

        // In the future, these words will be obtained from deserialized output,
        // rather than being crafted manually. I'll probably need GhostCell for
        // the self-referential parts

        // Push `mstar` into the execution context, basically
        // treating it as an "entry point"
        x.push_exec(RuntimeWord::VerbSeq(seq));

        loop {
            match x.step() {
                Ok(StepResult::Done) => break,
                Ok(StepResult::Working(ft)) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time
                    ft.exec(&mut x).unwrap();
                }
                Err(_e) => todo!(),
            }
        }

        let output = x.exchange_output();

        assert_eq!("***", &output);
    }
}
