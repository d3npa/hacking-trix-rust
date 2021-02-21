The idea is to let `miracle` run without segfaulting by editing the sections 
in the ELF metadata. `patcher` can be used to apply the modifications 
automatically. 

This technique works by locating a pt_note section in the ELF program header
and converting it to a pt_load section, which tells the loader to map a 
section of virtual memory. 

I don't fully understand why this is the case, but during testing I noticed
that it was always loading a full page (0x1000B) instead of just the data
I wanted, so I had to carefully pick a place in the binary where I could
write string data in such a way that I could map it to 0x12345.

The resulting `miracle` binary worked when I tested it on Void and Debian, 
but not on OpenBSD. I think the problem on OpenBSD may be that since the 
program data is organized differently, the offset I write the string 
could be important data.

```
$ make
rustc -C opt-level=3 -C debuginfo=0 -C relocation-model=static miracle.rs
rustc -C opt-level=3 -C debuginfo=0 patcher.rs
./patcher
$ ./miracle
It would take a miracle to make this work
Value at 0x12345: "MIRACLE"
$ make clean
rm miracle patcher
$
```
