.altmacro # 支持 .rept 等指令

/*
  __switch 函数用于任务切换。所谓任务切换主要是指任务上下文切换。
      # __switch(
    #     current_task_cx_ptr2: &*const TaskContext,
    #     next_task_cx_ptr2: &*const TaskContext
    # )
    为当前任务备份 TaskContext(从 SP 上分配新的 TaskContext，并将 ra/s0~s11 存入) ，写到 a0 指向的地址；
    并下一个 TaskContext(a1 指向) 换入运行流程
*/

.macro SAVE_SN n
    sd s\n, (\n+1)*8(sp)
.endm

.macro LOAD_SN n
    ld s\n, (\n+1)*8(sp)
.endm

    .section .text # 链接到代码段
    .globl __switch # 暴露才能被链接器发现
    # __switch(*curTCPtr, *nextTCPtr)，a0/a1 分别是两个参数地址
__switch:
    # 栈向下生长，此处分配出一个 TaskContext 的空间(13*8)
    addi sp, sp, -13*8 # 注意，这里的 SP 是当前任务的 SP，即保存任务的 TaskContext 是在任务自己的栈内分配的
    sd sp, 0(a0) # a0[0] = sp，这句可以解释为啥入参是二维指针
    sd ra, 0(sp) # 将 ra(当前返回地址) 写到 sp 指向的地址，即刚刚分配的 TaskContext 的 ra 域
    
    .set n, 0
    .rept 12 # 备份 s0~s11
        SAVE_SN %n
        .set n, n+1
    .endr

    ld sp, 0(a1) # 将下一个任务的 TaskContext 地址加载到 sp 中
    ld ra, 0(sp) # ra = *sp + 0
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n+1
    .endr
    # pop TaskContext
    addi sp, sp, 13*8 # 这里是弹出 next_task_cx 对应的 taskContext 栈，注意 sp 已被加载
    ret
