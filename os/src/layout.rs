
// 由 linker 指定，定义内核镜像的符号
extern "C" {
    /// 代码段开始
    fn stext(); 
    /// 代码段结束
    fn etext();
    /// 只读数据段
    fn srodata();
    fn erodata();

    /// 数据段
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();

    /// kernel 结束，后面内存可供应用使用
    fn ekernel();
    fn strampoline();
}