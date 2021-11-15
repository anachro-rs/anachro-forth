#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;

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

pub struct Runtime<T, F, Sdata, Sexec>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack2<T, F>,
    F: FuncSeq<T, F> + Clone,
    T: Clone,
{
    pub data_stk: Sdata,
    pub ret_stk: Sdata,
    pub flow_stk: Sexec,
    pub _pd_ty_t_f: PhantomData<(T, F)>,
    // cur_output: String,
}

impl<Sdata, Sexec, T, F> Runtime<T, F, Sdata, Sexec>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack2<T, F>,
    F: FuncSeq<T, F> + Clone,
    T: Clone,
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
            let cur = match self.flow_stk.last_mut() {
                Ok(frame) => frame,
                Err(_) => return Ok(StepResult::Done),
            };

            let mut jump = None;

            enum WhichToken<T, U> {
                Single(T),
                Seq(U),
            }

            let to_push = match cur.word.clone() {
                RefWord2::LiteralVal(lit) => {
                    self.data_stk.push(lit);
                    None
                }
                RefWord2::Verb(ft) => Some(WhichToken::Single(ft)),
                RefWord2::VerbSeq(seq) => {
                    // TODO: I should probably check for a difference
                    // between exactly one over-bounds (jump to end of seq),
                    // and overshooting (probably an engine error)
                    let ret = seq.get(cur.idx).map(WhichToken::Seq);
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
                    jump = Some(offset);
                    None
                }
                RefWord2::CondRelativeJump { offset, jump_on } => {
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
                Some(WhichToken::Single(ft)) => break 'oloop ft,
                Some(WhichToken::Seq(seq)) => {
                    self.flow_stk.push(RefExecCtx2 { idx: 0, word: seq });
                }
                None => {
                    self.flow_stk.pop()?;
                },
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
    use std::collections::BTreeMap;
    use core::marker::PhantomData;

    use super::*;


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

    // pub struct StdMapDict<'a, Sdata, Sexec>
    // where
    //     Sdata: Stack<Item = i32>,
    //     Sexec: ExecStack<'a, Sdata, Self>,
    // {
    //     data: BTreeMap<String, RefWord<'a, Sdata, Sexec, Self>>,
    // }

    // impl<'dict, Sdata, Sexec> RoDict<'dict, Sdata, Sexec> for StdMapDict<'dict, Sdata, Sexec>
    // where
    //     Sdata: Stack<Item = i32> + 'dict,
    //     Sexec: ExecStack<'dict, Sdata, Self> + 'dict,
    // {
    //     fn get<'a>(&self, name: &'a str) -> Option<&'dict RefWord<'dict, Sdata, Sexec, Self>> {
    //         todo!()
    //     }
    // }

    #[derive(Clone)]
    struct SeqTok {
        inner: Vec<RefWord2<Toker, SeqTok>>,
    }

    impl FuncSeq<Toker, SeqTok> for SeqTok {
        fn get(&self, idx: usize) -> Option<RefWord2<Toker, SeqTok>> {
            self.inner.get(idx).map(Clone::clone)
        }
    }

    #[derive(Clone)]
    struct Toker {
        bi: Builtin,
    }

    type Builtin = fn(
        &mut Runtime<
            Toker,
            SeqTok,
            StdVecStack<i32>,
            StdVecStack<
                RefExecCtx2<Toker, SeqTok>
            >
        >
    ) -> Result<(), Error>;

// pub fn bi_emit<T, F, Sdata, Sexec>(ctxt: &mut Runtime<T, F, Sdata, Sexec>) -> Result<(), Error>
// where
//    Sdata: Stack<Item = i32>,
//    Sexec: ExecStack2<T, F>,
//    F: FuncSeq<T, F> + Clone,
//    T: Clone,

    #[test]
    fn foo() {
        println!("yey");

        let mut ds = StdVecStack::new(Error::DataStackEmpty);
        let mut rs = StdVecStack::new(Error::DataStackEmpty);
        let mut fs = StdVecStack::new(Error::DataStackEmpty);

        let mut x = Runtime {
            data_stk: ds,
            ret_stk: rs,
            flow_stk: fs,
            _pd_ty_t_f: PhantomData,
        };

        x.push_exec(RefWord2::VerbSeq(SeqTok {
            inner: vec![
                RefWord2::LiteralVal(42),
                RefWord2::Verb(Toker { bi: builtins::bi_emit }),
            ]
        }));

        match x.step() {
            Ok(StepResult::Done) => todo!(),
            Ok(StepResult::Working(ft)) => {
                (ft.bi)(&mut x).unwrap();
            }
            Err(e) => todo!(),
        }

        panic!()
    }
}
