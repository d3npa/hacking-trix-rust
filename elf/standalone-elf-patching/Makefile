OPTS = -C opt-level=3 -C debuginfo=0

all:
	rustc $(OPTS) -C relocation-model=static miracle.rs
	rustc $(OPTS) patcher.rs
	./patcher
clean:
	rm miracle patcher
