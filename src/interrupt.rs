use crate::system::{CgbSystem, MappedReg};

#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    VBlank = 0,
    Stat,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    pub fn request(self, mem: &mut CgbSystem) {
        mem[MappedReg::If] |= 1 << self as usize;
    }
}
