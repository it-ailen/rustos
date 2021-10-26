
    .align 3 # 使用 1<<3(8) 字节指令对齐，xmas-elf解析 需要按8字节进行对齐
    .section .data
    .global _num_app
_num_app:
    .quad 21
    .quad app_0_start
    .quad app_1_start
    .quad app_2_start
    .quad app_3_start
    .quad app_4_start
    .quad app_5_start
    .quad app_6_start
    .quad app_7_start
    .quad app_8_start
    .quad app_9_start
    .quad app_10_start
    .quad app_11_start
    .quad app_12_start
    .quad app_13_start
    .quad app_14_start
    .quad app_15_start
    .quad app_16_start
    .quad app_17_start
    .quad app_18_start
    .quad app_19_start
    .quad app_20_start
    .quad app_20_end

    .global _app_names
_app_names:
    # 链接器会自动在每个字符串的结尾加入分隔符 \0
    .string "00pwoer_3"
    .string "01power_5"
    .string "02power_7"
    .string "03sleep"
    .string "exit"
    .string "fantastic_text"
    .string "forktest"
    .string "forktest2"
    .string "forktest_simple"
    .string "forktree"
    .string "hello_world"
    .string "initproc"
    .string "matrix"
    .string "pipe_large_test"
    .string "pipetest"
    .string "sleep"
    .string "sleep_simple"
    .string "stack_overflow"
    .string "user_shell"
    .string "usertests"
    .string "yield"

    .section .data
    .global app_0_start
    .global app_0_end
app_0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/00pwoer_3"
app_0_end:
        

    .section .data
    .global app_1_start
    .global app_1_end
app_1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/01power_5"
app_1_end:
        

    .section .data
    .global app_2_start
    .global app_2_end
app_2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/02power_7"
app_2_end:
        

    .section .data
    .global app_3_start
    .global app_3_end
app_3_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/03sleep"
app_3_end:
        

    .section .data
    .global app_4_start
    .global app_4_end
app_4_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/exit"
app_4_end:
        

    .section .data
    .global app_5_start
    .global app_5_end
app_5_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/fantastic_text"
app_5_end:
        

    .section .data
    .global app_6_start
    .global app_6_end
app_6_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest"
app_6_end:
        

    .section .data
    .global app_7_start
    .global app_7_end
app_7_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest2"
app_7_end:
        

    .section .data
    .global app_8_start
    .global app_8_end
app_8_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest_simple"
app_8_end:
        

    .section .data
    .global app_9_start
    .global app_9_end
app_9_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktree"
app_9_end:
        

    .section .data
    .global app_10_start
    .global app_10_end
app_10_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/hello_world"
app_10_end:
        

    .section .data
    .global app_11_start
    .global app_11_end
app_11_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/initproc"
app_11_end:
        

    .section .data
    .global app_12_start
    .global app_12_end
app_12_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/matrix"
app_12_end:
        

    .section .data
    .global app_13_start
    .global app_13_end
app_13_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/pipe_large_test"
app_13_end:
        

    .section .data
    .global app_14_start
    .global app_14_end
app_14_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/pipetest"
app_14_end:
        

    .section .data
    .global app_15_start
    .global app_15_end
app_15_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sleep"
app_15_end:
        

    .section .data
    .global app_16_start
    .global app_16_end
app_16_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sleep_simple"
app_16_end:
        

    .section .data
    .global app_17_start
    .global app_17_end
app_17_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/stack_overflow"
app_17_end:
        

    .section .data
    .global app_18_start
    .global app_18_end
app_18_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/user_shell"
app_18_end:
        

    .section .data
    .global app_19_start
    .global app_19_end
app_19_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/usertests"
app_19_end:
        

    .section .data
    .global app_20_start
    .global app_20_end
app_20_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/yield"
app_20_end:
        
