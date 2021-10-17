/* 批处理系统的应用程序是预先编译组织好的，不能动态加载。linker
负责将多个任务协调安排在不同的位置 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

/* BASE_ADDRESS = 0x80400000; /* 地址与内核镜像位置分开  */

BASE_ADDRESS = 0x0; /* 不再需要区分，启用虚拟地址后由内核负责处理地址冲突问题 */

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry) /* 入口，对应 lib.rs 中 _start */
        *(.text .text.*)
    }
    . = ALIGN(4K); /* 对齐是为了不同类型的数据放在不同页，它们间可能会以不同方式被访问 */
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    
    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } /* .data 与 .bss 的访问限制相同，所以不需要按页对齐，让它们紧凑排布即可 */
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