#![cfg_attr(not(test), no_std)]

use core::{fmt::Write, marker::PhantomData};

pub mod builtins;

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

// #[derive(Clone)]
// pub enum RefWord<'dict, Sdata, Sexec, Dict>
// where
//     Sdata: Stack<Item = i32> + 'dict,
//     Sexec: ExecStack<'dict, Sdata, Dict> + 'dict,
//     Dict: RoDict<'dict, Sdata, Sexec>,
// {
//     LiteralVal(i32),
//     Builtin {
//         name: &'dict str,
//         func: fn(&mut Runtime<Sdata, Sexec, Dict>) -> Result<(), Error>,
//     },
//     Compiled {
//         name: &'dict str,
//         data: &'dict [RefWord<'dict, Sdata, Sexec, Dict>],
//     },
//     UncondRelativeJump { offset: i32 },
//     CondRelativeJump { offset: i32, jump_on: bool },
// }

// #[derive(Debug, Clone)]
// pub struct FuncTok;

// #[derive(Debug, Clone)]
// pub struct FuncSeq;

pub trait FuncSeq<T, F>
where
    F: FuncSeq<T, F>,
    T: Clone,
    F: Clone,
{
    fn get(&self, _idx: usize) -> Option<RefWord2<T, F>>;
}

#[derive(Debug, Clone)]
pub enum RefWord2<T, F>
where
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    LiteralVal(i32),

    // TODO: These three are the same, and should all be reference
    // to some "owned" token, maybe GhostToken
    //
    // The stepper should return one of these tokens back to the
    // Runtime to be called against the current state. This will
    // "smuggle" the types of the actual runtime away from the
    // words themselves
    //
    // TODO: How to handle Sequences (Vec<FuncTok>, idx) and individuals (FuncTok)
    Verb(T),

    VerbSeq(F),

    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

pub struct RefExecCtx2<T, F>
where
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    pub idx: usize,
    pub word: RefWord2<T, F>,
}

// pub struct RefExecCtx<'dict, Sdata, Sexec, Dict>
// where
//     Sdata: Stack<Item = i32> + 'dict,
//     Sexec: ExecStack<'dict, Sdata, Dict> + 'dict,
//     Dict: RoDict<'dict, Sdata, Sexec>
// {
//     pub idx: usize,
//     pub word: RefWord<'dict, Sdata, Sexec, Dict>,
// }

pub struct Runtime<T, F, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack2<T, F>,
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
    Sexec: ExecStack2<T, F>,
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
        // AJM(TODO):
        // Okay, I *think* I need to turn this into a loop, that always sequences forwards until
        // until the next "syscall"/FuncTok is available.
        //
        // It'll go *something* like looping, but only return in cases where we have a FuncTok

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
                Ref(RefWord2<T, F>),
            }

            let to_push = match cur.word.clone() {
                RefWord2::LiteralVal(lit) => {
                    // println!("lit");
                    self.data_stk.push(lit);
                    None
                }
                RefWord2::Verb(ft) => {
                    // println!("verb");
                    Some(WhichToken::Single(ft))
                }
                RefWord2::VerbSeq(seq) => {
                    // println!("verbseq->{}", cur.idx);
                    // TODO: I should probably check for a difference
                    // between exactly one over-bounds (jump to end of seq),
                    // and overshooting (probably an engine error)
                    let ret = seq.get(cur.idx).map(WhichToken::Ref);
                    // println!("\tgot? {}", ret.is_some());
                    cur.idx += 1;
                    ret
                }
                // RefWord2::Builtin { func, .. } => {
                //     func(self)?;
                //     None
                // }
                // RefWord2::Compiled { data: words, .. } => {
                //     let ret = words.get(cur.idx).map(|t| { t.boop() });
                //     cur.idx += 1;
                //     ret
                // }
                RefWord2::UncondRelativeJump { offset } => {
                    // println!("ucrj");
                    jump = Some(offset);
                    None
                }
                RefWord2::CondRelativeJump { offset, jump_on } => {
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
                    self.flow_stk.push(RefExecCtx2 { idx: 0, word: rf });
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

    pub fn push_exec(&mut self, word: RefWord2<T, F>) {
        self.flow_stk.push(RefExecCtx2 { idx: 0, word });
    }
}

impl<Sdata, Sexec, T, F, O> Runtime<T, F, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack2<T, F>,
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
    fn last(&self) -> Result<&Self::Item, Error>;
    fn len(&self) -> usize;
    fn last_mut(&mut self) -> Result<&mut Self::Item, Error>;

    // TODO: This is suspicious...
    fn get_mut(&mut self, index: usize) -> Result<&mut Self::Item, Error>;
}

pub trait ExecStack2<T, F>
where
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    fn push(&mut self, data: RefExecCtx2<T, F>);
    fn pop(&mut self) -> Result<RefExecCtx2<T, F>, Error>;
    fn last(&self) -> Result<&RefExecCtx2<T, F>, Error>;
    fn last_mut(&mut self) -> Result<&mut RefExecCtx2<T, F>, Error>;
    fn len(&self) -> usize;

    // TODO: This is suspicious...
    fn get_mut(&mut self, index: usize) -> Result<&mut RefExecCtx2<T, F>, Error>;
}

// pub trait ExecStack<'dict, Sdata, Dict>: Sized
// where
//     Sdata: Stack<Item = i32> + 'dict,
//     Dict: RoDict<'dict, Sdata, Self>,
//     Self: 'dict,
// {
//     fn push(&mut self, data: RefExecCtx<'dict, Sdata, Self, Dict>);
//     fn pop(&mut self) -> Result<RefExecCtx<'dict, Sdata, Self, Dict>, Error>;
//     fn last(&self) -> Result<&RefExecCtx<'dict, Sdata, Self, Dict>, Error>;
//     fn last_mut(&mut self) -> Result<&mut RefExecCtx<'dict, Sdata, Self, Dict>, Error>;
//     fn len(&self) -> usize;

//     // TODO: This is suspicious...
//     fn get_mut(&mut self, index: usize) -> Result<&mut RefExecCtx<'dict, Sdata, Self, Dict>, Error>;
// }

// pub trait RoDict<'dict, Sdata, Sexec>: Sized + 'dict
// where
//     Sdata: Stack<Item = i32> + 'dict,
//     Sexec: ExecStack<'dict, Sdata, Self> + 'dict,
// {
//     fn get<'a>(&self, name: &'a str) -> Option<&'dict RefWord<'dict, Sdata, Sexec, Self>>;
// }

pub enum StepResult<T> {
    Done,
    Working(T),
}

#[cfg(test)]
mod test {
    use super::*;
    use core::marker::PhantomData;

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

    impl<T, F> ExecStack2<T, F> for StdVecStack<RefExecCtx2<T, F>>
    where
        F: FuncSeq<T, F> + Clone,
        T: Clone,
    {
        fn push(&mut self, data: RefExecCtx2<T, F>) {
            self.data.push(data)
        }
        fn pop(&mut self) -> Result<RefExecCtx2<T, F>, Error> {
            self.data.pop().ok_or(Error::FlowStackEmpty)
        }
        fn last(&self) -> Result<&RefExecCtx2<T, F>, Error> {
            self.data.last().ok_or(Error::FlowStackEmpty)
        }
        fn last_mut(&mut self) -> Result<&mut RefExecCtx2<T, F>, Error> {
            self.data.last_mut().ok_or(Error::FlowStackEmpty)
        }
        fn len(&self) -> usize {
            self.data.len()
        }

        // TODO: This is suspicious...
        fn get_mut(&mut self, index: usize) -> Result<&mut RefExecCtx2<T, F>, Error> {
            self.data.get_mut(index).ok_or(Error::FlowStackEmpty)
        }
    }

    #[derive(Clone)]
    struct SeqTok<'a> {
        inner: &'a [RefWord2<Toker<'a>, SeqTok<'a>>],
    }

    impl<'a> FuncSeq<Toker<'a>, SeqTok<'a>> for SeqTok<'a> {
        fn get(&self, idx: usize) -> Option<RefWord2<Toker<'a>, SeqTok<'a>>> {
            self.inner.get(idx).map(Clone::clone)
        }
    }

    #[derive(Clone)]
    struct Toker<'a> {
        bi: Builtin<'a>,
    }

    type Builtin<'a> = fn(
        &mut Runtime<
            Toker<'a>,
            SeqTok<'a>,
            StdVecStack<i32>,
            StdVecStack<RefExecCtx2<Toker<'a>, SeqTok<'a>>>,
            String,
        >,
    ) -> Result<(), Error>;

    #[test]
    fn foo() {
        // These are the only data structures required, and Runtime is generic over the
        // stacks, so I could easily use heapless::Vec as a backing structure as well
        let ds = StdVecStack::new(Error::DataStackEmpty);
        let rs = StdVecStack::new(Error::RetStackEmpty);
        let fs = StdVecStack::new(Error::FlowStackEmpty);

        // This is a generic Runtime type, I'll likely define two versions:
        // One with std-ability (for the host), and one no-std one, so users
        // wont have to deal with all the generic shenanigans
        let mut x = Runtime {
            data_stk: ds,
            ret_stk: rs,
            flow_stk: fs,
            _pd_ty_t_f: PhantomData,
            cur_output: String::new(),
        };

        // Manually craft a word, roughly:
        // : star 42 emit ;
        let pre_seq = [
            RefWord2::LiteralVal(42),
            RefWord2::Verb(Toker {
                bi: builtins::bi_emit,
            }),
        ];

        // Manually craft another word, roughly:
        // : mstar star -1 if star star then ;
        let seq = [
            RefWord2::VerbSeq(SeqTok { inner: &pre_seq }),
            RefWord2::LiteralVal(-1),
            RefWord2::CondRelativeJump {
                offset: 2,
                jump_on: false,
            },
            RefWord2::VerbSeq(SeqTok { inner: &pre_seq }),
            RefWord2::VerbSeq(SeqTok { inner: &pre_seq }),
        ];

        // In the future, these words will be obtained from deserialized output,
        // rather than being crafted manually. I'll probably need GhostCell for
        // the self-referential parts

        // Push `mstar` into the execution context, basically
        // treating it as an "entry point"
        x.push_exec(RefWord2::VerbSeq(SeqTok { inner: &seq }));

        loop {
            match x.step() {
                Ok(StepResult::Done) => break,
                Ok(StepResult::Working(ft)) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time
                    (ft.bi)(&mut x).unwrap();
                }
                Err(_e) => todo!(),
            }
        }

        let output = x.exchange_output();

        assert_eq!("***", &output);
    }
}
