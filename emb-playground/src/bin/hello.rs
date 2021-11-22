#![no_main]
#![no_std]

use emb_playground as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world!");

    emb_playground::exit()
}
