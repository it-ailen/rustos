FROM dinghao188/rcore-tutorial
LABEL maintainer="zouyalong" \
      version="1.1" \
      description="ubuntu 18.04 with tools for rustOS"

SHELL [ "/bin/bash", "-c" ]

RUN set -x \
      && apt-get install -y strace gdb

RUN export PATH=$PATH:/root/.cargo/bin \
      && rustup target add riscv64gc-unknown-none-elf \
      && cargo install cargo-binutils \
      && rustup component add llvm-tools-preview
