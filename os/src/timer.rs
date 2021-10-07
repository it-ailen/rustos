use riscv::register::time;

use crate::{config::CLOCK_FREQ, sbi::set_timer};

/// 时钟频率，100Hz
const TICKS_PER_SEC: usize = 100;

/// 读取 mtime 寄存器，得到当前 tick
pub fn get_time() -> usize {
    time::read()
}

pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / 1000)
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC)
}