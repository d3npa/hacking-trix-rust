RUSTCOPTS = -C opt-level=z -C debuginfo=0

all:
	rustc $(RUSTCOPTS) -C relocation-model=static target.rs
	nasm -o shellcode.o shellcode.s

clean:
	rm target shellcode.o
