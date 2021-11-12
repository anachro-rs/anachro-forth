use crate::{Context, Word};
use std::ops::Deref;
use std::fmt::Write;

fn bi_emit(ctxt: &mut Context) -> Result<(), ()> {
    let word = ctxt.data_stk.pop().ok_or(()).unwrap() as u32;
    let symbol = std::char::from_u32(word).unwrap_or('‽');
    ctxt.cur_output += &format!("{}", symbol);
    Ok(())
}

fn bi_coredump(ctxt: &mut Context) -> Result<(), ()> {
    writeln!(&mut ctxt.cur_output, "DATA STACK:").unwrap();
    writeln!(&mut ctxt.cur_output, "{:08X?}", ctxt.data_stk).unwrap();
    writeln!(&mut ctxt.cur_output, "").unwrap();

    writeln!(&mut ctxt.cur_output, "RETURN/CONTROL STACK:").unwrap();
    writeln!(&mut ctxt.cur_output, "{:08X?}", ctxt.ret_stk).unwrap();
    writeln!(&mut ctxt.cur_output, "").unwrap();

    writeln!(&mut ctxt.cur_output, "DICT:").unwrap();
    for (key, word) in ctxt.dict.iter() {
        write!(&mut ctxt.cur_output, "  - {:?} => ", key).unwrap();
        let word: &Word = word.deref();
        match word {
            Word::Builtin(_) => writeln!(&mut ctxt.cur_output, "(builtin)"),
            Word::Compiled(ucw) => writeln!(&mut ctxt.cur_output, "(compiled, len: {})", ucw.len()),
            Word::LiteralVal(lit) => writeln!(&mut ctxt.cur_output, "Literal: {}", lit),
            Word::CondRelativeJump { .. } => writeln!(&mut ctxt.cur_output, "COND RELATIVE JUMP! TODO!"),
            Word::UncondRelativeJump { .. } => writeln!(&mut ctxt.cur_output, "UNCOND RELATIVE JUMP! TODO!"),
        }.unwrap();
    }

    Ok(())
}

fn bi_pop(ctxt: &mut Context) -> Result<(), ()> {
    writeln!(&mut ctxt.cur_output, "{}", ctxt.data_stk.pop().ok_or(()).unwrap()).unwrap();
    Ok(())
}

fn bi_cr(ctxt: &mut Context) -> Result<(), ()> {
    writeln!(&mut ctxt.cur_output, "").unwrap();
    Ok(())
}

pub fn bi_lt(ctxt: &mut Context) -> Result<(), ()> {
    let val2 = ctxt.data_stk.pop().ok_or(()).unwrap();
    let val1 = ctxt.data_stk.pop().ok_or(()).unwrap();
    ctxt.data_stk.push(if val1 < val2 { -1 } else { 0 });
    Ok(())
}

pub fn bi_gt(ctxt: &mut Context) -> Result<(), ()> {
    let val2 = ctxt.data_stk.pop().ok_or(()).unwrap();
    let val1 = ctxt.data_stk.pop().ok_or(()).unwrap();
    ctxt.data_stk.push(if val1 > val2 { -1 } else { 0 });
    Ok(())
}

pub fn bi_retstk_push(ctxt: &mut Context) -> Result<(), ()> {
    let val = ctxt.data_stk.pop().ok_or(()).unwrap();
    ctxt.ret_stk.push(val);
    Ok(())
}

pub fn bi_retstk_pop(ctxt: &mut Context) -> Result<(), ()> {
    let val = ctxt.ret_stk.pop().ok_or(()).unwrap();
    ctxt.data_stk.push(val);
    Ok(())
}

fn bi_eq(ctxt: &mut Context) -> Result<(), ()> {
    let val1 = ctxt.data_stk.pop().ok_or(()).unwrap();
    let val2 = ctxt.data_stk.pop().ok_or(()).unwrap();
    ctxt.data_stk.push(if val1 == val2 { -1 } else { 0 });
    Ok(())
}

pub fn bi_add(ctxt: &mut Context) -> Result<(), ()> {
    let val1 = ctxt.data_stk.pop().ok_or(()).unwrap();
    let val2 = ctxt.data_stk.pop().ok_or(()).unwrap();
    ctxt.data_stk.push(val1.wrapping_add(val2));
    Ok(())
}

fn bi_dup(ctxt: &mut Context) -> Result<(), ()> {
    let val1 = ctxt.data_stk.last().ok_or(()).unwrap().clone();
    ctxt.data_stk.push(val1);
    Ok(())
}

fn bi_serdump(ctxt: &mut Context) -> Result<(), ()> {
    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::LiteralVal(val) = word {
            writeln!(
                &mut ctxt.cur_output,
                "LIT\t{}\t0x{:08X}\t0x{:016X}",
                name, *val, word as *const _ as usize
            ).unwrap();
        }
    }

    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::Builtin(_) = word {
            writeln!(
                &mut ctxt.cur_output,
                "BLT\t{}\t{:016X}",
                name,
                word as *const _ as usize
            ).unwrap();
        }
    }

    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::Compiled(words) = word {
            writeln!(
                &mut ctxt.cur_output,
                "CMP\t{}\t{:016X}\t{:016X?}",
                name,
                word as *const _ as usize,
                words
                    .iter()
                    .map(|w| {
                        // TODO: I should probably directly print literals
                        let word: &Word = w.deref();
                        word as *const _ as usize
                    })
                    .collect::<Vec<_>>()
            ).unwrap();
        }
    }

    Ok(())
}

pub static BUILT_IN_WORDS: &[(&str, Word)] = &[
    ("emit", Word::Builtin(bi_emit)),
    (".", Word::Builtin(bi_pop)),
    ("cr", Word::Builtin(bi_cr)),
    (">r", Word::Builtin(bi_retstk_push)),
    ("r>", Word::Builtin(bi_retstk_pop)),
    ("=", Word::Builtin(bi_eq)),
    ("<", Word::Builtin(bi_lt)),
    (">", Word::Builtin(bi_gt)),
    ("dup", Word::Builtin(bi_dup)),
    ("+", Word::Builtin(bi_add)),
    // TODO: This requires the ability to modify the input stream!
    //
    // This is supposed to return the address of the NEXT word in the
    // input stream
    //
    // ("'", Word::Builtin(bi_tick))

    // ( @ is the "load operator )  ok
    // ( ! is the "store operator" )  ok

    // Constants store a VALUE in the dict, which will be pushed on the stack
    //
    // I *think*:
    //
    // 5 CONSTANT X
    //
    // is equivalent to:
    //
    // : X 5 ;

    // Variables store the value, and put the ADDRESS on the stack when invoked
    //
    // I *think*:
    //
    // 0 VARIABLE ZERO
    //
    // is equvalent to:
    //
    // ).unwrap().unwrap()

    // Debug
    ("serdump", Word::Builtin(bi_serdump)),
    ("coredump", Word::Builtin(bi_coredump)),
];
