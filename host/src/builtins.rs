use crate::{Context, Error, Word};
use std::fmt::Write;
use std::ops::Deref;
use anachro_forth_core::{Stack, Runtime, ExecStack};

fn bi_emit<'stor, Sdata, Sexec>(ctxt: &mut Runtime<'stor, Sdata, Sexec>) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack<'stor, Sdata>,
 {
    let word = ctxt.data_stk.pop()? as u32;
    let symbol = std::char::from_u32(word).unwrap_or('‽');
    // ctxt.cur_output += &format!("{}", symbol);
    Ok(())
}

// fn bi_coredump<'stor, Sdata, Sexec>(ctxt: &mut Runtime<'stor, Sdata, Sexec>) -> Result<(), Error>
// where
//     Sdata: Stack<Item = i32>,
//     Sexec: ExecStack<'stor, Sdata>,
//  {
//     // writeln!(&mut ctxt.cur_output, "DATA STACK:")?;
//     // writeln!(&mut ctxt.cur_output, "{:08X?}", ctxt.data_stk)?;
//     // writeln!(&mut ctxt.cur_output, "")?;

//     // writeln!(&mut ctxt.cur_output, "RETURN/CONTROL STACK:")?;
//     // writeln!(&mut ctxt.cur_output, "{:08X?}", ctxt.ret_stk)?;
//     // writeln!(&mut ctxt.cur_output, "")?;

//     // writeln!(&mut ctxt.cur_output, "FLOW STACK LEN:")?;
//     // writeln!(&mut ctxt.cur_output, "{}", ctxt.flow_stk.len())?;
//     // writeln!(&mut ctxt.cur_output, "")?;

//     // writeln!(&mut ctxt.cur_output, "DICT:")?;
//     for (key, word) in ctxt.dict.data.iter() {
//         // write!(&mut ctxt.cur_output, "  - {:?} => ", key)?;
//         let word: &Word = word.deref();
//         match word {
//             // Word::Builtin { .. } => writeln!(&mut ctxt.cur_output, "(builtin)"),
//             // Word::Compiled { name, data } => writeln!(&mut ctxt.cur_output, "(compiled: {}, len: {})", name, data.len()),
//             // Word::LiteralVal(lit) => writeln!(&mut ctxt.cur_output, "Literal: {}", lit),
//             Word::CondRelativeJump { .. } => {
//                 // writeln!(&mut ctxt.cur_output, "COND RELATIVE JUMP! TODO!")
//             }
//             Word::UncondRelativeJump { .. } => {
//                 // writeln!(&mut ctxt.cur_output, "UNCOND RELATIVE JUMP! TODO!")
//             }
//         }?;
//     }

//     Ok(())
// }


// pub fn bi_retstk_2dup<'stor, Sdata, Sexec>(ctxt: &mut Runtime<'stor, Sdata, Sexec>) -> Result<(), Error>
// where
//     Sdata: Stack<Item = i32>,
//     Sexec: ExecStack<'stor, Sdata>,
//  {
//     let len = ctxt.ret_stk.len();
//     if len < 2 {
//         return Err(Error::RetStackEmpty);
//     }

//     for _ in 0..2 {
//         let bot = ctxt
//             .ret_stk
//             .data
//             .get(len - 2)
//             .ok_or(Error::RetStackEmpty)?
//             .clone();
//         ctxt.ret_stk.data.push(bot);
//     }

//     Ok(())
// }

pub fn bi_retstk_swap<'stor, Sdata, Sexec>(ctxt: &mut Runtime<'stor, Sdata, Sexec>) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack<'stor, Sdata>,
 {
    let len = ctxt.ret_stk.len();
    if len < 2 {
        return Err(Error::RetStackEmpty);
    }

    let top = ctxt.ret_stk.pop()?;
    let bot = ctxt.ret_stk.pop()?;
    ctxt.ret_stk.push(top);
    ctxt.ret_stk.push(bot);

    Ok(())
}

pub fn bi_priv_loop<'stor, Sdata, Sexec>(ctxt: &mut Runtime<'stor, Sdata, Sexec>) -> Result<(), Error>
where
    Sdata: Stack<Item = i32>,
    Sexec: ExecStack<'stor, Sdata>,
 {
    let lmt = ctxt.ret_stk.pop()?;
    let mut idx = ctxt.ret_stk.pop()?;

    println!("lmt: {}, idx: {}", lmt, idx);

    idx = idx.checked_add(1).ok_or(Error::BadMath)?;

    if idx == lmt {
        ctxt.data_stk.push(-1);
    } else {
        ctxt.data_stk.push(0);
        ctxt.ret_stk.push(idx);
        ctxt.ret_stk.push(lmt);
    }

    Ok(())
}

// fn bi_serdump<'stor, Sdata, Sexec>(ctxt: &mut Runtime<'stor, Sdata, Sexec>) -> Result<(), Error>
// where
//     Sdata: Stack<Item = i32>,
//     Sexec: ExecStack<'stor, Sdata>,
//  {
//     for (name, word) in ctxt.dict.data.iter() {
//         let word: &Word = word.deref();
//         if let Word::LiteralVal(val) = word {
//             // writeln!(
//             //     // &mut ctxt.cur_output,
//             //     "LIT\t{}\t0x{:08X}\t0x{:016X}",
//             //     name, *val, word as *const _ as usize
//             // )?;
//         }
//     }

//     for (name, word) in ctxt.dict.data.iter() {
//         let word: &Word = word.deref();
//         if let Word::Builtin { .. } = word {
//             // writeln!(
//             //     // &mut ctxt.cur_output,
//             //     "BLT\t{}\t{:016X}",
//             //     name, word as *const _ as usize
//             // )?;
//         }
//     }

//     for (_name, word) in ctxt.dict.data.iter() {
//         let word: &Word = word.deref();
//         if let Word::Compiled { name, data: words } = word {
//             // writeln!(
//             //     // &mut ctxt.cur_output,
//             //     "CMP\t{}\t{:016X}\t{:016X?}",
//             //     name,
//             //     word as *const _ as usize,
//             //     words
//             //         .iter()
//             //         .map(|w| {
//             //             // TODO: I should probably directly print literals
//             //             let word: &Word = w.deref();
//             //             word as *const _ as usize
//             //         })
//             //         .collect::<Vec<_>>()
//             // )?;
//         }
//     }

//     Ok(())
// }

// pub static BUILT_IN_WORDS: &[(&'static str, fn(&mut Context) -> Result<(), Error>)] = &[
//     ("emit", bi_emit),
//     (".", bi_pop),
//     ("cr", bi_cr),
//     (">r", bi_retstk_push),
//     ("r>", bi_retstk_pop),
//     ("=", bi_eq),
//     ("<", bi_lt),
//     (">", bi_gt),
//     ("dup", bi_dup),
//     ("+", bi_add),
//     // TODO: This requires the ability to modify the input stream!
//     //
//     // This is supposed to return the address of the NEXT word in the
//     // input stream
//     //
//     // ("'", bi_tick)

//     // ( @ is the "load operator )  ok
//     // ( ! is the "store operator" )  ok

//     // Constants store a VALUE in the dict, which will be pushed on the stack
//     //
//     // I *think*:
//     //
//     // 5 CONSTANT X
//     //
//     // is equivalent to:
//     //
//     // : X 5 ;

//     // Variables store the value, and put the ADDRESS on the stack when invoked
//     //
//     // I *think*:
//     //
//     // 0 VARIABLE ZERO
//     //
//     // is equvalent to:
//     //
//     // ).unwrap().unwrap()

//     // Debug
//     ("serdump", bi_serdump),
//     ("coredump", bi_coredump),
// ];
