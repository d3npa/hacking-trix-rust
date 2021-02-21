use std::{fs, io};
use std::io::prelude::*;
use std::io::SeekFrom;

macro_rules! read_u64 {
    ( $file:ident, $offset:expr ) => ({
        $file.seek(SeekFrom::Start($offset))?;
        let mut buffer = [0u8; 8];
        $file.read(&mut buffer)?;
        u64::from_ne_bytes(buffer)
    });
}

macro_rules! read_u32 {
    ( $file:ident, $offset:expr ) => ({
        $file.seek(SeekFrom::Start($offset))?;
        let mut buffer = [0u8; 4];
        $file.read(&mut buffer)?;
        u32::from_ne_bytes(buffer)
    });
}

macro_rules! read_u16 {
    ( $file:ident, $offset:expr ) => ({
        $file.seek(SeekFrom::Start($offset))?;
        let mut buffer = [0u8; 2];
        $file.read(&mut buffer)?;
        u16::from_ne_bytes(buffer)
    });
}

fn find_pt_note(f: &mut fs::File) -> io::Result<Option<u64>> {
    // Parse ELF header (note this method calls lots of seeks)
    // Better to read the full header into an array and parse that instead
    let     e_phoff = read_u64!(f, 0x20);
    let e_phentsize = read_u16!(f, 0x36);
    let     e_phnum = read_u16!(f, 0x38);

    // Find pt_note if there is one
    // Maybe a cool place to implement an iterator?
    for i in 0..e_phnum {
        let phent_offset = e_phoff + (e_phentsize * i) as u64;
        f.seek(SeekFrom::Start(phent_offset))?;

        let p_type = read_u32!(f, phent_offset);

        if p_type == 4 {
            return Ok(Some(phent_offset));
        }
    }

    Ok(None)
}

fn main() -> io::Result<()> {
    let mut f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("./miracle")?;

    let phent_offset = find_pt_note(&mut f)?
        .unwrap_or_else(|| {
            eprintln!("Did not find pt_note section");
            std::process::exit(1);
        });

    // Find suitable place for string
    // We want the string to be loaded to 0x12345 exactly, so we have to make
    // sure to write it to a place that, once loaded, will overlap perfectly.
    // There should be lots of string data above the section headers which
    // are safe to overwrite...
    let e_shoff = read_u64!(f, 0x28);
    let target = e_shoff & !0xfff; // clear 3 lowest bits

    // Set p_type to pt_load to force loading
    f.seek(SeekFrom::Start(phent_offset))?;
    f.write(&(1u32.to_ne_bytes()))?;

    // Overwrite p_offset
    f.seek(SeekFrom::Current(0x4))?;
    f.write(&(target.to_ne_bytes()))?;

    // Overwrite p_vaddr
    // Cursor is already positioned from last write
    f.write(&(0x12000u64.to_ne_bytes()))?;

    // Patch string
    f.seek(SeekFrom::Start(target + 0x345))?;
    f.write("MIRACLE".as_bytes())?;

    Ok(())
}