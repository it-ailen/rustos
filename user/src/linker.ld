/* 批处理系统的应用程序是预先编译组织好的，不能动态加载。linker
负责将多个任务协调安排在不同的位置 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80400000; /* 地址与内核镜像位置分开 */

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry) /* 入口，对应 lib.rs 中 _start */
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        start_bss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        end_bss = .;
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}