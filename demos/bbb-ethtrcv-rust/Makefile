BINARY := target/armv7a-none-eabihf/release/bbb-ethtrcv-rust

.PHONY: all clean

all: $(BINARY).bin

$(BINARY).bin: $(BINARY)
	rust-objcopy -O binary $< $@
	@echo "Binary: $@ ($$(wc -c < $@) bytes)"

$(BINARY):
	cargo build --release

clean:
	cargo clean
