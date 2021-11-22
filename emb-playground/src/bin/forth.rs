#![no_main]
#![no_std]

use emb_playground as _; // global logger + panicking-behavior + memory layout
use anachro_forth_core::{RuntimeWord, VerbSeqInner, nostd_rt::NoStdContext, ser_de::SerDictFixed};
use groundhog_nrf52::GlobalRollingTimer;
use groundhog::RollingTimer;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello!");
    // let prog: &mut [u8] = &mut [3, 2, 2, 2, 42, 1, 1, 2, 1, 1, 4, 5, 4, 2, 1, 1, 1, 2, 2, 1, 3, 3, 2, 1, 1, 2, 2, 1, 2, 2, 1, 7, 1, 4, 101, 109, 105, 116, 0];
    let prog: &mut [u8] = &mut [2, 2, 2, 7, 4, 64, 66, 15, 1, 1, 1, 1, 1, 2, 1, 1, 2, 1, 1, 2, 2, 1, 3, 1, 1, 6, 4, 253, 255, 255, 255, 15, 2, 2, 62, 114, 9, 80, 82, 73, 86, 95, 76, 79, 79, 80, 0];

    let board = nrf52840_hal::pac::Peripherals::take().unwrap();

    groundhog_nrf52::GlobalRollingTimer::init(board.TIMER0);

    let loaded: SerDictFixed<4, 16, 4> = postcard::from_bytes_cobs(prog).unwrap();
    let mut ns_ctxt: NoStdContext<32, 16, 128, 4, 16> = NoStdContext::from_ser_dict(&loaded);

    let temp_compiled = RuntimeWord::VerbSeq(VerbSeqInner::from_word(1));
    ns_ctxt.rt.push_exec(temp_compiled.clone());

    let timer = GlobalRollingTimer::new();
    let now = timer.get_ticks();
    ns_ctxt.run_blocking().unwrap();
    let elapsed = timer.micros_since(now);

    defmt::info!("lol: {}", elapsed);

    emb_playground::exit()
}
