// rustc -C relocation-model=static -C opt-level=3 -C debuginfo=0 miracle.rs
// Disable PIE by compiling with relocation-model=static
// Patch PT_NOTE: map 0x37000 to 0x12000
// Patch string @0x37345 (search for 'ustomUn') to 'MIRACLE'

fn main() {
    let ptr = 0x12345usize as *const [u8; 7];

    unsafe {
        println!("It would take a miracle to make this work");
        if let Ok(magic) = String::from_utf8((*ptr).to_vec()) {
            println!("Value at {:?}: {:?}", ptr, magic);
        }
    }
}