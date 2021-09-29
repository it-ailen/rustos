OUTPUT_ARCH(riscv) /* 目标平台，此链接器只在 riscv 编译时被使用 */
ENTRY(_start) /* 指定全局入口 */
BASE_ADDRESS = 0x80200000; /* bootloader(RustSBI) 指定的加载位置 */

SECTIONS /* 使用链接脚本配置内存布局，而不使用默认布局 */
{
    . = BASE_ADDRESS;/* . 表示当前地址 */
    skernel = .; /* start kernel ?*/

    stext = .;
    .text : { /* 代码段，列表。冒号前表示段名。花括号内按照放置顺序描述将所有输入目标文件的哪些段放在这个段中 */
        *(.text.entry)
        *(.text .text.*) /* 表示将目标文件的 .text 段放到各 .text.* 的位置 */
    }
    . = ALIGN(4K); /* 按页对齐 */
    etext = .; /* 代码段结束 */

    srodata = .; /* 只读数据，常量、字符串字面值？ */
    .rodata : {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4K); /* 按页对齐 */
    erodata = .;

    sdata = .; /* 数据段，全局静态变量 */
    .data : {
        *(.data .data.*)
    }
    . = ALIGN(4K);
    edata = .;

    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .; /* kernel 段结束 */

    /DISCARD/ : {
        *(.eh_frame)
    }
}