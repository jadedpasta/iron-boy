#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    VBlank = 0,
    Stat,
    Timer,
    Serial,
    Joypad,
}

pub struct InterruptState {
    pub enable: u8,
    pub flags: u8,
}

impl InterruptState {
    pub fn new() -> Self {
        Self { enable: 0, flags: 0 }
    }

    pub fn request(&mut self, interrupt: Interrupt) {
        self.flags |= 1 << interrupt as usize;
    }

    pub fn pop(&mut self) -> Option<u8> {
        let pending = self.enable & self.flags;
        let bit = pending.trailing_zeros() as u8;
        if bit > 7 {
            // No interrupts are pending.
            return None;
        }
        // Toggle off the flag bit to mark the interrupt as handled.
        self.flags ^= 1 << bit;
        Some(bit)
    }
}
