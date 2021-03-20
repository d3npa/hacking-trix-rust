# PT_NOTE to PT_LOAD ELF injector example

I read about a technique on the [SymbolCrash blog](https://www.symbolcrash.com/2019/03/27/pt_note-to-pt_load-injection-in-elf/) for injecting shellcode into an ELF binary by converting a `PT_NOTE` in the Program Headers into a `PT_LOAD`. I thought this sounded interesting and I didn't know a lot about ELF, so I took it as an opportunity to learn many new things at once.

For this project I created a small, very incomplete library I called [mental_elf](https://github.com/d3npa/mental-elf) which makes parsing and writing ELF metadata easier. I think the library code is very straight-forward and easy to understand, so I won't talk about it any more here. Let's focus on the infection technique instead :)

## Overview

As implied by the title, this infection technique involves converting an ELF's `PT_NOTE` program header into a `PT_LOAD` in order to run shellcode. The infection boils down to three steps:

- Append the shellcode to the end of the ELF file
- Load the shellcode to a specific address in virtual memory 
- Change the ELF's entry point to the above address so the shellcode is executed first

The shellcode should also be patched for each ELF such that it jumps back to the host ELF's original entry point, allowing the host to execute normally after the shellcode is finished. 

Shellcode may be loaded into virtual memory via a `PT_LOAD` header. Inserting a new program header into the ELF file would likely break many offsets throughout the binary, however it is usually possible to repurpose a `PT_NOTE` header without breaking the binary. Here is a note about the Note Section in the [ELF Specification](http://www.skyfree.org/linux/references/ELF_Format.pdf):

> Note information is optional.  The presence of note information does not affect a program’s ABI conformance, provided the information does not affect the program’s execution behavior.  Otherwise, the program does not conform to the ABI and has undefined behavior

Here are two caveats I became aware of:

- This simplistic technique will not work with Position Independent Executables (PIE). 
- The Go language runtime actually expects a valid `PT_NOTE` section containing version information in order to run, so this technique cannot be used with Go binaries.

Note: PIE can be disabled in cc with `-no-pie` or in rustc with `-C relocation-model=static`.

## Shellcode

The shellcode provided is written for the Netwide ASseMbler (NASM). Make sure to install `nasm` before running the Makefile! 

To create shellcode suitable for this injection, there are a couple of things to keep in mind. Section 3.4.1 of the [AMD64 System V ABI](https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.95.pdf) says that the `rbp`, `rsp`, and `rdx` registers must be set to correct values before entry. This can be achieved by ordinary pushing and popping around the shellcode. My shellcode doesn't touch `rbp` or `rsp`, and setting `rdx` to zero before returning also worked.

The shellcode also needs to be patched so it can actually jump back to the host's original entry point after finishing. To make patching easier, shellcode can be designed to run off the end of the file, either by being written top-to-bottom, or jumping to an empty label at the end:

```nasm
main_tasks:
    ; ...
    jmp finish
other_tasks:
    ; ...
finish:
```

With this design, patching is as easy as appending a jump instruction. In x86_64 however, `jmp` cannot take a 64bit operand - instead the destination is stored in rax and then a `jmp rax` is made. This rust snippet patches a `shellcode` byte vector to append a jump to `entry_point`:

```rust
fn patch_jump(shellcode: &mut Vec<u8>, entry_point: u64) {
    // Store entry_point in rax
    shellcode.extend_from_slice(&[0x48u8, 0xb8u8]);
    shellcode.extend_from_slice(&entry_point.to_ne_bytes());
    // Jump to address in rax
    shellcode.extend_from_slice(&[0xffu8, 0xe0u8]);
}
```

## Infector

The infector itself is in `src/main.rs`. It's written in an easy to follow top-to-bottom format, so if you understood the overview it should be very clear. I also added comments to help. The code uses my [mental_elf](https://github.com/d3npa/mental-elf) library to abstract away the details of reading/writing the file, so that it's easier to see the technique.

In summary, the code

- Takes in 2 CLI parameters: the ELF target and a shellcode file
- Reads in the ELF and Program headers from the ELF file
- Patches the shellcode with a `jmp` to the original entry point
- Appends the patched shellcode the ELF
- Finds a `PT_NOTE` program header and converts it to `PT_LOAD`
- Changes the ELF's entry point to the start of the shellcode
- Saves the altered header structures back into the ELF file

When an infected ELF file is run, the ELF loader will map several sections of the ELF file into virtual memory - our crated `PT_LOAD` will make sure our shellcode is loaded and executable. The ELF's entry point then starts the shellcode's execution. Then the shellcode ends, it will then jump to the original entry point, allowing the binary to run its original code.

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

This was a very fun project where I learned so much about ELF, parsing binary structures in Rust, and viruses in general! Thanks to netspooky, sblip, TMZ, and others at tmp.out for teaching me, helping me debug and motivating me to do this project <3