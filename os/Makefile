DOCKER_NAME = target/riscv64gc-unknown-none-elf/debug/os
.PHONY: run .os disasm

run: .os
	qemu-riscv64 ${DOCKER_NAME}

.os: ${DOCKER_NAME}
	cargo build

disasm:
	rust-objdump -S ${DOCKER_NAME}
