#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    VBlank = 0,
    Stat,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    pub fn request(self, if_reg: &mut u8) {
        *if_reg |= 1 << self as usize;
    }
}
