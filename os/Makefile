TARGET := riscv64gc-unknown-none-elf
MODE ?= release

OUTPUT_DIR := target/$(TARGET)/$(MODE)
KERNEL_ELF := $(OUTPUT_DIR)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := $(OUTPUT_DIR)/asm
FS_IMG := ../user/target/$(TARGET)/$(MODE)/fs.img

BOARD ?= qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin

# Kernel entry
ifeq ($(BOARD),qemu)
	KERNEL_ENTRY_PA := 0x80200000
endif

# binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# BUILD_FLAGS := --features "board_$(BOARD)"

build: $(KERNEL_BIN) $(FS_IMG)

$(KERNEL_BIN): kernel
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel: 
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
ifeq ($(MODE),release)
	cargo build --release $(BUILD_FLAGS)
else
	cargo build  $(BUILD_FLAGS)
endif
	@rm src/linker.ld

clean:
	cargo clean

disasm: kernel
	$(OBJDUMP) -x $(KERNEL_ELF) | less

run: build
ifeq ($(BOARD),qemu)
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
endif

$(FS_IMG): $(APPS)
	@cd ../user && make build
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/

$(APPS):

debug: $(KERNEL_BIN)
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-S -s

gdb:
	RUST_GDB=riscv64-unknown-elf-gdb rust-gdb -iex 'target remote :1234' $(KERNEL_ELF)

gdb_old:
	riscv64-unknown-elf-gdb -iex 'target remote :1234' $(KERNEL_ELF)

.PHONY: build run disasm clean kernel debug gdb
