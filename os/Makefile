TARGET := riscv64gc-unknown-none-elf
MODE := release

OUTPUT_DIR := target/$(TARGET)/$(MODE)
KERNEL_ELF := $(OUTPUT_DIR)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := $(OUTPUT_DIR)/asm

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

build: $(KERNEL_BIN)

$(KERNEL_BIN): kernel
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel: 
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	cargo build --release
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
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
endif

debug:
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
