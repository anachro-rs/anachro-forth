use crate::*;

pub fn bi_emit<T, F, Sdata, Sexec>(ctxt: &mut Runtime<T, F, Sdata, Sexec>) -> Result<(), Error>
where
   Sdata: Stack<Item = i32>,
   Sexec: ExecStack2<T, F>,
   F: FuncSeq<T, F> + Clone,
   T: Clone,
 {
    let word = ctxt.data_stk.pop()? as u32;
    let symbol = core::char::from_u32(word).unwrap_or('‽');
    #[cfg(test)]
    println!("{:?}", symbol);
    // ctxt.cur_output += &format!("{}", symbol);
    Ok(())
}
