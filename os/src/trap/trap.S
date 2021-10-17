.altmacro # 告诉编译器支持 .rept

# 保存通用寄存器 SAVE_GP(n)，其中 n 为 Xn 寄存器编号
.macro SAVE_GP n
    sd x\n, \n*8(sp) # store dword
.endm

# 从栈上读取 Xn 的值写回对应 Xn 寄存器
.macro LOAD_GP n
    ld x\n, \n*8(sp) # load dword
.endm
    .section .text.trampoline # 陷入上下文切换属于跳板部分
    .globl __alltraps # 必须声明成 globl 才能被链接器发现
    .globl __restore

.align 2
    # alltraps 执行前 CPU 处于 S 模式
__alltraps:
    # 此指令前，sp 指向用户栈，sscrath 指向内核栈，执行后 sp <= sscratch && sscratch <= sp
    # 实际是起到了交换 sp/sscratch 的作用
    # sscratch 存的是上一次 __restore 备份的的用户空间 sp
    csrrw sp, sscratch, sp
    # now sp->kernel stack, sscratch->user stack
    # allocate a TrapContext on kernel stack
    # 上一句执行后，sp 指向内核栈，而 sscratch 指向用户栈
    # 为保存用户应用程序分配上下文。sp 向下增长，34byte 为 trapContext 的大小
    # 此时 sp 在内核，所以分配 的是内核栈
    # save general-purpose registers
    # 备份能用寄存器
    sd x1, 1*8(sp) # 从低到高，跳过 x0(zero)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp) # 跳过 x2(sp)，后面保存，因为下面可能还会从栈上分配
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
    # 保存 sstatus 和 sepc
    # t0 等寄存器已在前面被备份了，可以正常使用
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp) # 备份 sstatus
    sd t1, 33*8(sp) # 备份 sepc
    # read user stack from sscratch and save it on the kernel stack
    csrr t2, sscratch # sscratch 目前是用户栈栈顶
    sd t2, 2*8(sp) # 将用户栈栈顶指针保存到内核栈内
    ld t0, 34*8(sp) # 将 kernel_satp 加载到 t0
    ld t1, 36*8(sp) # 加载 trap_handler 到 t1
    ld sp, 35*8(sp) # 加载 kernel_sp 到 sp

    csrw satp, t0 # 切换到内核空间
    sfence.vma # 地址空间切换，需要清空当前 TLB 内容
    # 因为 call 时采用的是相对位移（虚拟地址），而我们会使用 linker.ld 将跳板
    # 页面（本文档代码）放在虚拟地址的最高页，相对位移无法索引到 trap_handler
    # 所以这里直接使用跳转指令
    jr t1 # 跳转到 trap_handler。问题：为啥不使用 call trap_handler?


    # call trap_handler 返回后，执行的下一条指令即 restore
    # __restore(a0, a1)
    # a0: trapContext 在应用地址空间的位置，对所有应用都是相同的
    # a1: 即将回到的应用的地址空间。注意，这里完成了任务切换，所以这是下一个应用的地址空间。
    # restore 执行前 CPU 处于 S 模式
__restore:
    # 交换当前 satp 和 a1, a1 为要切换的用户任务，当前 satp 为内核地址空间
    csrw satp, a1
    sfence.vma # 地址空间切换，需要清空当前 TLB 内容
    csrw sscratch, a0 # 将 a0 中的值暂存到 sscratch, 存入后 trap 时就能用了
    mv sp, a0 # sp=a0, 即此时 sp 指向 trapContext 的用户空间虚地址
    # restore sstatus/sepc
    ld t0, 32*8(sp) # 对应 TrapContext.sstatus
    ld t1, 33*8(sp) # 对应 TrapContext.sepc
    csrw sstatus, t0
    csrw sepc, t1
    
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # 切换回用户栈
    ld sp, 2*8(sp)
    sret # 从 S 模式返回到 U 特权级继续运行应用程序控制流
