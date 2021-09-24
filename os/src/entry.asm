    .section .text.entry # 指定段名为 .text.entry，对应 linker.ld 中第一部分，所以这个 asm 会被放在 .text 的首部，即 0x8020000
    .globl _start # 声明全局符号 _start，在 linker.ld 中将它指定为了整个 os 镜像的入口
_start:
    la sp, boot_stack_top # 设置 sp 寄存器
    call rust_main # 初始化栈结束，跳到 rust 入口

    .section .bss.stack # 此栈被放入 linker.ld 指定的 .bss 段的低地址空间
    .globl boot_stack # 全局符号 boot_stack，表示栈底
boot_stack:
    .space 4096*16 # 64KB 栈空间，栈向下生长
    .globl boot_stack_top
boot_stack_top: