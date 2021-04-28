RUSTCOPTS = -C opt-level=z -C debuginfo=0

all:
	rustc $(RUSTCOPTS) target.rs
	nasm -o shellcode.o shellcode.s
	nasm -o jump_to_start.o jump_to_start.s

clean:
	rm target shellcode.o jump_to_start.o
