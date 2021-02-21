/*
 * This program demonstrates how to build and write out a ROP chain.
 * The code can be adapted to write to a socket, a file, or other stream.
 * Compile with `rustc -C opt-level=3 -C debuginfo=0 rop-chain`
 */

use std::{io, mem};
use std::io::Write;

fn main() {
    const SIZE: usize = 4;
    let array: [u64; SIZE] = [
        0xaaaaaaaaaaaaaaaa,
        0x0, 
        0x0, 
        0xbbbbbbbbbbbbbbbb, 
    ];

    let data: [u8; SIZE * 8] = unsafe { mem::transmute(array) };
    let _ = io::stdout().write(&data);
}