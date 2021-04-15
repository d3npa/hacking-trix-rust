all:
	cd files && make && cd ..
	cargo run --release files/target files/shellcode.o
	@echo 'Done! Run target with: `./files/target`'
clean:
	cd files && make clean && cd ..
	cargo clean
