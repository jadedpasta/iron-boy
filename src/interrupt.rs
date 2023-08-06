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

    fn pending_bits(&self) -> u8 {
        self.enable & self.flags
    }

    pub fn pending(&self) -> bool {
        self.pending_bits() != 0
    }

    pub fn pop(&mut self) -> Option<u8> {
        let bit = self.pending_bits().trailing_zeros() as u8;
        if bit > 7 {
            // No interrupts are pending.
            return None;
        }
        // Toggle off the flag bit to mark the interrupt as handled.
        self.flags ^= 1 << bit;
        Some(bit)
    }
}
