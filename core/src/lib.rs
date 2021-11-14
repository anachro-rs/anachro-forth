#![cfg_attr(not(test), no_std)]

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

#[derive(Clone)]
pub enum RefWord<'stor, Sdata, Sexec>
where
    Sdata: Stack<Item = i32> + 'stor,
    Sexec: ExecStack<'stor, Sdata> + 'stor,
{
    LiteralVal(i32),
    Builtin {
        name: &'stor str,
        func: fn(&mut Runtime<Sdata, Sexec>) -> Result<(), Error>,
    },
    Compiled {
        name: &'stor str,
        data: &'stor [RefWord<'stor, Sdata, Sexec>],
    },
    UncondRelativeJump { offset: i32 },
    CondRelativeJump { offset: i32, jump_on: bool },
}

pub struct RefExecCtx<'stor, Sdata, Sexec>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack<'stor, Sdata>,
{
    pub idx: usize,
    pub word: RefWord<'stor, Sdata, Sexec>,
}

pub struct Runtime<'stor, Sdata, Sexec>
where
    Sdata: Stack<Item = i32> + 'stor,
    Sexec: ExecStack<'stor, Sdata> + 'stor,
{
    pub data_stk: &'stor mut Sdata,
    pub ret_stk: &'stor mut Sdata,
    pub flow_stk: &'stor mut Sexec,
    // cur_output: String,
}

impl<'stor, Sdata, Sexec> Runtime<'stor, Sdata, Sexec>
where
    Sdata: Stack<Item = i32> + Clone,
    Sexec: ExecStack<'stor, Sdata> + Clone,
{
    pub fn step(&mut self) -> Result<StepResult, Error> {
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

    fn step_inner(&mut self) -> Result<StepResult, Error> {
        let cur = match self.flow_stk.last_mut() {
            Ok(frame) => frame,
            Err(_) => return Ok(StepResult::Done),
        };

        let mut jump = None;

        let to_push = match cur.word {
            RefWord::LiteralVal(lit) => {
                self.data_stk.push(lit);
                None
            }
            RefWord::Builtin { func, .. } => {
                func(self)?;
                None
            }
            RefWord::Compiled { data: words, .. } => {
                let ret = words.get(cur.idx).map(Clone::clone);
                cur.idx += 1;
                ret
            }
            RefWord::UncondRelativeJump { offset } => {
                jump = Some(offset);
                None
            }
            RefWord::CondRelativeJump { offset, jump_on } => {
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

        if let Some(push) = to_push {
            self.push_exec(push);
        } else {
            let _ = self.flow_stk.pop();
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

        Ok(StepResult::Working)
    }

    pub fn push_exec(&mut self, word: RefWord<'stor, Sdata, Sexec>) {
        self.flow_stk.push(RefExecCtx { idx: 0, word });
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

pub trait ExecStack<'stor, Sdata>: Sized
where
    Sdata: Stack<Item = i32>,
{
    fn push(&mut self, data: RefExecCtx<'stor, Sdata, Self>);
    fn pop(&mut self) -> Result<RefExecCtx<'stor, Sdata, Self>, Error>;
    fn last(&self) -> Result<&RefExecCtx<'stor, Sdata, Self>, Error>;
    fn len(&self) -> usize;
    fn last_mut(&mut self) -> Result<&mut RefExecCtx<'stor, Sdata, Self>, Error>;

    // TODO: This is suspicious...
    fn get_mut(&mut self, index: usize) -> Result<&mut RefExecCtx<'stor, Sdata, Self>, Error>;
}

pub trait RoDict<'stor, Sdata, Sexec>: Sized + 'stor
where
    Sdata: Stack<Item = i32> + 'stor,
    Sexec: ExecStack<'stor, Sdata> + 'stor,
{
    fn get<'a>(&self, name: &'a str) -> Option<&'stor RefWord<'stor, Sdata, Sexec>>;
}

pub enum StepResult {
    Done,
    Working,
}
