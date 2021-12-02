use crate::*;
use core::fmt::Write;

pub fn bi_emit<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let word = ctxt.data_stk.pop()? as u32;
    let symbol = core::char::from_u32(word).unwrap_or('‽');
    write!(&mut ctxt.cur_output, "{}", symbol).map_err(|_| Error::OutputFormat)
}

pub fn bi_pop<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    writeln!(&mut ctxt.cur_output, "{}", ctxt.data_stk.pop()?)?;
    Ok(())
}

pub fn bi_drop<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let _ = ctxt.data_stk.pop()?;
    Ok(())
}

pub fn bi_rot<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let top = ctxt.data_stk.pop()?;
    let mid = ctxt.data_stk.pop()?;
    let bot = ctxt.data_stk.pop()?;

    ctxt.data_stk.push(mid)?;
    ctxt.data_stk.push(top)?;
    ctxt.data_stk.push(bot)?;

    Ok(())
}

pub fn bi_cr<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    writeln!(&mut ctxt.cur_output)?;
    Ok(())
}

pub fn bi_lt<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val2 = ctxt.data_stk.pop()?;
    let val1 = ctxt.data_stk.pop()?;
    ctxt.data_stk.push(if val1 < val2 { -1 } else { 0 })?;
    Ok(())
}

pub fn bi_gt<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val2 = ctxt.data_stk.pop()?;
    let val1 = ctxt.data_stk.pop()?;
    ctxt.data_stk.push(if val1 > val2 { -1 } else { 0 })?;
    Ok(())
}

pub fn bi_retstk_push<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val = ctxt.data_stk.pop()?;
    ctxt.ret_stk.push(val)?;
    Ok(())
}

pub fn bi_retstk_pop<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val = ctxt.ret_stk.pop()?;
    ctxt.data_stk.push(val)?;
    Ok(())
}

pub fn bi_eq<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val1 = ctxt.data_stk.pop()?;
    let val2 = ctxt.data_stk.pop()?;
    ctxt.data_stk.push(if val1 == val2 { -1 } else { 0 })?;
    Ok(())
}

pub fn bi_add<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val1 = ctxt.data_stk.pop()?;
    let val2 = ctxt.data_stk.pop()?;
    ctxt.data_stk.push(val1.wrapping_add(val2))?;
    Ok(())
}

pub fn bi_dup<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val1 = *ctxt.data_stk.last()?;
    ctxt.data_stk.push(val1)?;
    Ok(())
}

pub fn bi_retstk_dup<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val1 = *ctxt.ret_stk.last()?;
    ctxt.ret_stk.push(val1)?;
    Ok(())
}

pub fn bi_2dup<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let val1 = ctxt.data_stk.pop()?;
    let val2 = ctxt.data_stk.pop()?;
    ctxt.data_stk.push(val2)?;
    ctxt.data_stk.push(val1)?;
    ctxt.data_stk.push(val2)?;
    ctxt.data_stk.push(val1)?;
    Ok(())
}

pub fn bi_retstk_swap<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let top = ctxt.ret_stk.pop()?;
    let bot = ctxt.ret_stk.pop()?;
    ctxt.ret_stk.push(top)?;
    ctxt.ret_stk.push(bot)?;

    Ok(())
}

pub fn bi_swap<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let top = ctxt.data_stk.pop()?;
    let bot = ctxt.data_stk.pop()?;
    ctxt.data_stk.push(top)?;
    ctxt.data_stk.push(bot)?;

    Ok(())
}

pub fn bi_pick<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let top = ctxt.data_stk.pop()?;
    let val = *ctxt.data_stk.peek_back(top.try_into().map_err(|_| Error::DataStackUnderflow)?)?;
    ctxt.data_stk.push(val)?;

    Ok(())
}

pub fn bi_roll<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let top = ctxt.data_stk.pop()?;
    let val = ctxt.data_stk.pop_back(top.try_into().map_err(|_| Error::DataStackUnderflow)?)?;
    ctxt.data_stk.push(val)?;

    Ok(())
}

pub fn bi_priv_loop<BuiltinTok, SeqTok, Sdata, Sexec, O>(
    ctxt: &mut Runtime<BuiltinTok, SeqTok, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<BuiltinTok, SeqTok>,
    SeqTok: Clone,
    BuiltinTok: Clone,
    O: Write,
{
    let lmt = ctxt.ret_stk.pop()?;
    let mut idx = ctxt.ret_stk.pop()?;

    idx = idx.checked_add(1).ok_or(Error::BadMath)?;

    if idx == lmt {
        ctxt.data_stk.push(-1)?;
    } else {
        ctxt.data_stk.push(0)?;
        ctxt.ret_stk.push(idx)?;
        ctxt.ret_stk.push(lmt)?;
    }

    Ok(())
}
