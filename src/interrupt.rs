use crate::memory::{Memory, MappedReg};

#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    VBlank = 0,
    Stat,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    pub fn request(self, mem: &mut Memory) {
        mem[MappedReg::If] |= 1 << self as usize;
    }
}
