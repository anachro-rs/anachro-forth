use crate::*;
use core::fmt::Write;

pub fn bi_emit<T, F, Sdata, Sexec, O>(
    ctxt: &mut Runtime<T, F, Sdata, Sexec, O>,
) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecutionStack<T, F>,
    F: FuncSeq<T, F> + Clone,
    T: Clone,
    O: Write,
{
    let word = ctxt.data_stk.pop()? as u32;
    let symbol = core::char::from_u32(word).unwrap_or('‽');
    write!(&mut ctxt.cur_output, "{}", symbol).map_err(|_| Error::OutputFormat)
}
