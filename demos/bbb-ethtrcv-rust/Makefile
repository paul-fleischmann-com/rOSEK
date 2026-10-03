BINARY   := target/armv7a-none-eabihf/release/bbb-ethtrcv-rust
CC       := arm-none-eabi-gcc
CFLAGS   := -mcpu=cortex-a8 -mfpu=neon -mfloat-abi=hard \
            -ffreestanding -nostdlib -Os \
            -Iinclude
LDFLAGS  := -T linker.ld -nostdlib

C_OBJ    := target/armv7a-none-eabihf/release/c_demo.o

.PHONY: all clean

all: $(BINARY).bin

# Raw binary for U-Boot 'go' command
$(BINARY).bin: $(BINARY)
	rust-objcopy -O binary $< $@
	@echo "Binary: $@ ($$(wc -c < $@) bytes)"

# Rust binary (Rust entry point, exports EthTrcv C API)
$(BINARY): $(C_OBJ)
	cargo build --release

# C demo object — compiled separately, linked via Cargo (build.rs not needed
# for a standalone demo; link manually if integrating into a pure-C project)
$(C_OBJ): c_demo/main.c include/EthTrcv.h
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	cargo clean
	rm -f $(C_OBJ)
