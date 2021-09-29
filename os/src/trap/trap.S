.altmacro # 告诉编译器支持 .rept

# 保存通用寄存器 SAVE_GP(n)，其中 n 为 Xn 寄存器编号
.macro SAVE_GP n
    sd x\n, \n*8(sp) # store dword
.endm

# 从栈上读取 Xn 的值写回对应 Xn 寄存器
.macro LOAD_GP n
    ld x\n, \n*8(sp) # load dword
.endm
    .section .text
    .globl __alltraps # 必须声明成 globl 才能被链接器发现
    .globl __restore 

.align 2
__alltraps:
    # 此指令前，sp 指向用户栈，sscrath 指向内核栈，执行后 sp <= sscratch && sscratch <= sp
    # 实际是起到了交换 sp/sscratch 的作用 
    csrrw sp, sscratch, sp
    # 上一句执行后，sp 指向内核栈，而 sscratch 指向用户栈
    # 为保存用户应用程序分配上下文。sp 向下增长，34byte 为 trapContext 的大小
    # 此时 sp 在内核，所以分配 的是内核栈
    addi sp, sp, -34 * 8
    # 
    # 备份能用寄存器
    sd x1, 1*8(sp) # 从低到高，跳过 x0(zero)
    sd x3, 3*8(sp) # 跳过 x2(sp)，后面保存，因为下面可能还会从栈上分配
    # skip tp(x4), application does not use it
    # for n = 5, i = 0; i < 27; i++, n++ 
    .set n, 5
    .rept 27 # 32 - 5，n 从5 开始，循环 27次
        SAVE_GP %n
        .set n, n+1
    .endr

    # 保存 sstatus 和 sepc
    # t0 等寄存器已在前面被备份了，可以正常使用
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp) # 备份 sstatus
    sd t1, 33*8(sp) # 备份 sepc
    csrr t2, sscratch # sscratch 目前是用户栈栈顶
    sd t2, 2*8(sp) # 将用户栈栈顶指针保存到内核栈内
    mv a0, sp # sp 此时为 &mut TrapContext
    # trap_handler 依赖寄存器的值，但在 call 过程中，寄存器可能被更改。
    # 所以使用 TrapContext 在内存中备份这些寄存器以供 trap_handler 使用
    call trap_handler

    # call trap_handler 返回后，执行的下一条指令即 restore
__restore:
    # 将 sp 恢复为调用前的 a0，所以 trap_handler 需要保证执行前后其值不变
    mv sp, a0
    ld t0, 32*8(sp) # 对应 TrapContext.sstatus
    ld t1, 33*8(sp) # 对应 TrapContext.sepc
    ld t2, 2*8(sp) # 对应 TrapContext.X2，即 sp(用户栈栈顶)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # 恢复通用寄存器
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # 回收内核栈中用户栈 TrapContext
    addi sp, sp, 34*8
    # Trap 逆操作，将 sp 换回用户栈
    csrrw sp, sscratch, sp
    sret # 回到 U 特权级继续运行应用程序控制流