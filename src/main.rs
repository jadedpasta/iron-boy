mod cpu;
mod memory;

use std::{env, fs};

use cpu::Cpu;
use memory::Memory;

fn main() {
    let file_name = env::args().nth(1).unwrap();
    let mut mem = Memory::new(fs::read(file_name).unwrap());

    let mut cpu = Cpu::default();
    loop {
        cpu.execute(&mut mem);
    }
}
