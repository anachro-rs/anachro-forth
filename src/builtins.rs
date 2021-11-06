use crate::{Context, Word};
use std::{
    io::{stdout, Write},
    ops::Deref,
};

fn bi_emit(ctxt: &mut Context) -> Result<(), ()> {
    let word = ctxt.data_stk.pop().ok_or(()).unwrap() as u32;
    let symbol = std::char::from_u32(word).unwrap_or('‽');
    print!("{}", symbol);
    stdout().flush().map_err(drop)
}

fn bi_coredump(ctxt: &mut Context) -> Result<(), ()> {
    println!("DATA STACK:");
    println!("{:08X?}", ctxt.data_stk);
    println!("");

    println!("RETURN/CONTROL STACK:");
    println!("{:08X?}", ctxt.ret_stk);
    println!("");

    println!("DICT:");
    for (key, word) in ctxt.dict.iter() {
        print!("  - {:?} => ", key);
        let word: &Word = word.deref();
        match word {
            Word::Builtin(_) => println!("(builtin)"),
            Word::Compiled(ucw) => println!("(compiled, len: {})", ucw.len()),
            Word::LiteralVal(lit) => println!("Literal: {}", lit),
            Word::CondRelativeJump { .. } => println!("COND RELATIVE JUMP! TODO!"),
            Word::UncondRelativeJump { .. } => println!("UNCOND RELATIVE JUMP! TODO!"),
        }
    }

    Ok(())
}

fn bi_pop(ctxt: &mut Context) -> Result<(), ()> {
    println!("{}", ctxt.data_stk.pop().ok_or(()).unwrap());
    Ok(())
}

fn bi_cr(_ctxt: &mut Context) -> Result<(), ()> {
    println!("");
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
            println!(
                "LIT\t{}\t0x{:08X}\t0x{:016X}",
                name, *val, word as *const _ as usize
            );
        }
    }

    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::Builtin(_) = word {
            println!("BLT\t{}\t{:016X}", name, word as *const _ as usize);
        }
    }

    for (name, word) in ctxt.dict.iter() {
        let word: &Word = word.deref();
        if let Word::Compiled(words) = word {
            println!(
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
            );
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
