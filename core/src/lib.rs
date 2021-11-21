#![cfg_attr(not(any(test, feature = "std")), no_std)]

use core::{fmt::Write, marker::PhantomData};

pub mod builtins;

#[cfg(any(test, feature = "std"))]
pub mod std_rt;

pub mod nostd_rt;

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

pub enum WhichToken<BuiltinTok, SeqTok>
where
    SeqTok: Clone,
    BuiltinTok: Clone,
{
    Single(BuiltinTok),
    Ref(VerbSeqInner<SeqTok>),
}

#[derive(Debug, Clone)]
pub struct VerbSeqInner<SeqTok>
where
    SeqTok: Clone,
{
    pub tok: SeqTok,
    pub idx: usize,
}

impl<SeqTok> VerbSeqInner<SeqTok>
where
    SeqTok: Clone,
{
    pub fn from_word(tok: SeqTok) -> Self {
        Self {
            tok,
            idx: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuntimeWord<BuiltinTok, SeqTok>
where
    SeqTok: Clone,
    BuiltinTok: Clone,
{
    LiteralVal(i32),

    // TODO: Blend these somehow?
    Verb(BuiltinTok),
    VerbSeq(VerbSeqInner<SeqTok>),

    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

impl<BuiltinTok, SeqTok> RuntimeWord<BuiltinTok, SeqTok>
where
    SeqTok: Clone,
    BuiltinTok: Clone,
{
    pub fn as_seq_inner(&mut self) -> Result<&mut VerbSeqInner<SeqTok>, Error> {
        match self {
            RuntimeWord::VerbSeq(ref mut seq) => Ok(seq),
            _ => Err(Error::InternalError),
        }
    }
}

pub struct Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    pub data_stk: Sdata,
    pub ret_stk: Sdata,
    pub flow_stk: Sexec,
    pub _pd_ty_t_f: PhantomData<(BuiltinTok, SeqTok)>,
    cur_output: O,
}

impl<Sdata, Sexec, BuiltinTok, SeqTok, O> Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    pub fn step(&mut self) -> Result<StepResult<BuiltinTok, SeqTok>, Error> {
        match self.step_inner() {
            Ok(r) => Ok(r),
            Err(e) => {
                while self.flow_stk.pop().is_ok() {}
                while self.data_stk.pop().is_ok() {}
                while self.ret_stk.pop().is_ok() {}
                Err(e)
            }
        }
    }

    fn step_inner(&mut self) -> Result<StepResult<BuiltinTok, SeqTok>, Error> {
        let ret = 'oloop: loop {
            // TODO: I should set a limit to the max number of loop
            // iterations that are made here! Or maybe go back to
            // yielding at each step
            let cur = match self.flow_stk.last_mut() {
                Ok(frame) => frame,
                Err(_) => return Ok(StepResult::Done),
            };

            let mut jump = None;

            let to_push = match cur {
                RuntimeWord::LiteralVal(lit) => {
                    // println!("lit");
                    self.data_stk.push(*lit);
                    None
                }
                RuntimeWord::Verb(ft) => {
                    // println!("verb");
                    Some(WhichToken::Single(ft.clone()))
                }
                RuntimeWord::VerbSeq(ref mut seq) => {
                    // println!("verbseq->{}", cur.idx);
                    // TODO: I should probably check for a difference
                    // between exactly one over-bounds (jump to end of seq),
                    // and overshooting (probably an engine error)
                    let ret = Some(WhichToken::Ref(seq.clone()));
                    // println!("\tgot? {}", ret.is_some());
                    seq.idx += 1;
                    ret
                }
                RuntimeWord::UncondRelativeJump { offset } => {
                    // println!("ucrj");
                    jump = Some(*offset);
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
                    let do_jump = (topvar == 0) ^ *jump_on;

                    // println!("topvar: {}, jump_on: {}", topvar, jump_on);

                    if do_jump {
                        // println!("Jumping!");
                        jump = Some(*offset);
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
                    break 'oloop WhichToken::Single(ft);
                }
                Some(WhichToken::Ref(rf)) => {
                    // println!("FLOWPUSH");
                    break 'oloop WhichToken::Ref(rf);
                }
                None => {
                    // println!("FLOWPOP");
                    self.flow_stk.pop()?;
                }
            }

            if let Some(jump) = jump {
                // We just popped off the jump command, so now we are back in
                // the "parent" frame.

                let new_cur = self
                    .flow_stk
                    .last_mut()?
                    .as_seq_inner()?;

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

    pub fn provide_seq_tok(&mut self, seq: Option<RuntimeWord<BuiltinTok, SeqTok>>) -> Result<(), Error> {
        if let Some(mut word) = seq {
            if let Ok(wd) = word.as_seq_inner() {
                assert_eq!(wd.idx, 0);
                wd.idx = 0;
            }
            self.flow_stk.push(word);
        } else {
            self.flow_stk.pop()?;
        }
        Ok(())
    }

    pub fn push_exec(&mut self, mut word: RuntimeWord<BuiltinTok, SeqTok>) {
        if let Ok(wd) = word.as_seq_inner() {
            assert_eq!(wd.idx, 0);
            wd.idx = 0;
        }
        self.flow_stk.push(word);
    }
}

impl<Sdata, Sexec, BuiltinTok, SeqTok, O> Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
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

pub trait ExecutionStack<BuiltinTok, SeqTok>
where
    SeqTok: Clone,
    BuiltinTok: Clone,
{
    fn push(&mut self, data: RuntimeWord<BuiltinTok, SeqTok>);
    fn pop(&mut self) -> Result<RuntimeWord<BuiltinTok, SeqTok>, Error>;
    fn last_mut(&mut self) -> Result<&mut RuntimeWord<BuiltinTok, SeqTok>, Error>;
}

pub enum StepResult<BuiltinTok, SeqTok>
where
    SeqTok: Clone,
    BuiltinTok: Clone,
{
    Done,
    Working(WhichToken<BuiltinTok, SeqTok>),
}

#[cfg(test)]
mod std_test {
    use super::*;
    use crate::std_rt::*;
    use std::sync::Arc;
    use std::collections::BTreeMap;

    #[test]
    fn foo() {
        let mut x = new_runtime();

        let mut fs_map: BTreeMap<String, StdFuncSeq> = BTreeMap::new();

        // Manually craft a word, roughly:
        // : star 42 emit ;
        fs_map.insert(
            "star".into(),
            StdFuncSeq {
                inner: Arc::new(vec![
                    NamedStdRuntimeWord {
                        word: RuntimeWord::LiteralVal(42),
                        name: "42".into(),
                    },
                    NamedStdRuntimeWord {
                        word: RuntimeWord::Verb(BuiltinToken::new(builtins::bi_emit)),
                        name: "emit".into(),
                    },
                ]),
            });

        // Manually craft another word, roughly:
        // : mstar star -1 if star star then ;
        fs_map.insert(
            "mstar".into(),
            StdFuncSeq {
                inner: Arc::new(vec![
                    NamedStdRuntimeWord {
                        word: RuntimeWord::VerbSeq(VerbSeqInner::from_word("star".to_string())),
                        name: "star".into(),
                    },
                    NamedStdRuntimeWord {
                        word: RuntimeWord::LiteralVal(-1),
                        name: "-1".into(),
                    },
                    NamedStdRuntimeWord {
                        word: RuntimeWord::CondRelativeJump {
                            offset: 2,
                            jump_on: false,
                        },
                        name: "UCRJ".into(),
                    },
                    NamedStdRuntimeWord {
                        word: RuntimeWord::VerbSeq(VerbSeqInner::from_word("star".to_string())),
                        name: "star".into(),
                    },
                    NamedStdRuntimeWord {
                        word: RuntimeWord::VerbSeq(VerbSeqInner::from_word("star".to_string())),
                        name: "star".into(),
                    },
                ]),
            });

        // // In the future, these words will be obtained from deserialized output,
        // // rather than being crafted manually. I'll probably need GhostCell for
        // // the self-referential parts

        // // Push `mstar` into the execution context, basically
        // // treating it as an "entry point"
        x.push_exec(RuntimeWord::VerbSeq(VerbSeqInner::from_word("mstar".to_string())));

        loop {
            match x.step() {
                Ok(StepResult::Done) => break,
                Ok(StepResult::Working(WhichToken::Single(ft))) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time
                    println!("EXEC!");
                    ft.exec(&mut x).unwrap();
                }
                Ok(StepResult::Working(WhichToken::Ref(rtw))) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time

                    let c = fs_map
                        .get(&rtw.tok)
                        .and_then(|n| n.inner.get(rtw.idx))
                        .map(|n| n.clone().word);

                    if c.is_some() {
                        println!("Push some! {} {}", rtw.tok, rtw.idx);
                    } else {
                        println!("Push none! {} {}", rtw.tok, rtw.idx);
                    }

                    x.provide_seq_tok(c).unwrap();

                }
                Err(_e) => todo!(),
            }
        }

        let output = x.exchange_output();

        assert_eq!("***", &output);
    }
}


#[cfg(test)]
mod nostd_test {
    use super::*;
    use crate::nostd_rt::*;
    use heapless::{Vec, String};

    #[test]
    fn foo() {

        let mut deser_dict: Vec<
            Vec<
                RuntimeWord<
                    BuiltinToken<32, 16, 256>,
                    usize,
                >,
                8
            >,
            8
        > = Vec::new();

        // Manually craft a word, roughly:
        // : star 42 emit ;
        deser_dict.push(
            {
                let mut new: Vec<RuntimeWord<BuiltinToken<32, 16, 256>, usize>, 8> = Vec::new();
                new.push(RuntimeWord::LiteralVal(42)).ok();
                new.push(RuntimeWord::Verb(BuiltinToken::new(builtins::bi_emit))).ok();
                new
            }
        ).ok();


        // Manually craft another word, roughly:
        // : mstar star -1 if star star then ;
        deser_dict.push(
            {
                let mut new: Vec<RuntimeWord<BuiltinToken<32, 16, 256>, usize>, 8> = Vec::new();
                new.push(RuntimeWord::VerbSeq(VerbSeqInner::from_word(0))).ok();
                new.push(RuntimeWord::LiteralVal(-1)).ok();
                new.push(RuntimeWord::CondRelativeJump {
                    offset: 2,
                    jump_on: false,
                }).ok();
                new.push(RuntimeWord::VerbSeq(VerbSeqInner::from_word(0))).ok();
                new.push(RuntimeWord::VerbSeq(VerbSeqInner::from_word(0))).ok();
                new
            }
        ).ok();

        // Not mutable anymore
        let idx = deser_dict;

        let mut x = new_runtime::<32, 16, 256>();

        // BuiltinToken<DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
        // usize,
        // HVecStack<i32, DATA_SZ>,
        // HVecStack<
        //     RuntimeWord<
        //         BuiltinToken<DATA_SZ, FLOW_SZ, OUTBUF_SZ>,
        //         usize,
        //     >,
        //     FLOW_SZ,
        // >,
        // String<OUTBUF_SZ>,

        let _sz = core::mem::size_of::<Runtime<
            BuiltinToken<32, 16, 256>,
            usize,
            HVecStack<i32, 32>,
            HVecStack<
                RuntimeWord<
                    BuiltinToken<32, 16, 256>,
                    usize,
                >,
                16,
            >,
            String<256>,
        >>();
        // <32, 16, 256> -> 856 (on a 64-bit machine)
        // assert_eq!(856, _sz);

        // In the future, these words will be obtained from deserialized output,
        // rather than being crafted manually. I'll probably need GhostCell for
        // the self-referential parts

        // Push `mstar` into the execution context, basically
        // treating it as an "entry point"
        x.push_exec(RuntimeWord::VerbSeq(
            // Insert `mstar`, which is deser_dict[1]
            VerbSeqInner { tok: 1, idx: 0 }
        ));

        loop {
            match x.step() {
                Ok(StepResult::Done) => {
                    // println!("DONE!");
                    break
                }
                Ok(StepResult::Working(WhichToken::Single(ft))) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time
                    // println!("EXEC!");
                    ft.exec(&mut x).unwrap();
                }
                Ok(StepResult::Working(WhichToken::Ref(rtw))) => {
                    // The runtime yields back at every call to a "builtin". Here, I
                    // call the builtin immediately, but I could also yield further up,
                    // to be resumed at a later time

                    let c = idx
                        .get(rtw.tok)
                        .and_then(|n| n.get(rtw.idx))
                        .map(|n| n.clone());

                    // if c.is_some() {
                    //     println!("Push some! {} {}", rtw.tok, rtw.idx);
                    // } else {
                    //     println!("Push none! {} {}", rtw.tok, rtw.idx);
                    // }

                    x.provide_seq_tok(c).unwrap();

                }
                Err(_e) => todo!(),
            }
        }

        let output = x.exchange_output();

        assert_eq!("***", &output);
    }
}
