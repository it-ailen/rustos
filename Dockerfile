FROM dinghao188/rcore-tutorial
LABEL maintainer="zouyalong" \
      version="1.1" \
      description="ubuntu 18.04 with tools for rustOS"

RUN set -x \
      && apt-get update \
      && apt-get install -y strace 

RUN set -x \
      && rustup target add riscv64gc-unknown-none-elf \
      && cargo install cargo-binutils \
      && rustup component add llvm-tools-preview
