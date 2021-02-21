# `PT_NOTE` to `PT_LOAD` elf injection

More info: https://www.symbolcrash.com/2019/03/27/pt_note-to-pt_load-injection-in-elf/

This technique involves converting a `PT_NOTE` program section to a `PT_LOAD` section.
Since it relies on editing the ELF header's `e_entry` it is not stealthy and also doesn't work with PIE executatbles.
To use against Rust programs, one must pass `-C relocation-model=static` to the compiler.

The provided code also assumes you are targetting a little-endian 64-bit ELF.

```
$ ./run
rustc -C opt-level=z -C debuginfo=0 -C relocation-model=static target.rs
nasm -o shellcode.o shellcode.s
    Finished release [optimized] target(s) in 0.00s
     Running `target/release/pt_note-to-pt_load-injector files/target files/shellcode.o`
Found PT_NOTE; Changing to PT_LOAD
Done! Run target with: `./files/target`
$ 
$ ./files/target
dont tell anyone im here
hello world!
$
```
