use crate::memory::{MappedReg, Memory};

use super::Cpu;

impl Cpu {
    pub(super) fn ei(&mut self) {
        self.enable_interrupts_timer = 2;
    }

    pub(super) fn di(&mut self) {
        self.enable_interrupts_timer = 0;
        self.interrupts_enabled = false;
    }

    pub(super) fn reti(&mut self, mem: &Memory) {
        self.interrupts_enabled = true;
        self.ret(mem);
    }

    pub(super) fn update_interrupt_timer(&mut self) {
        if self.enable_interrupts_timer > 0 {
            self.enable_interrupts_timer -= 1;
            if self.enable_interrupts_timer == 0 {
                self.interrupts_enabled = true;
            }
        }
    }

    pub(super) fn handle_interrupts(&mut self, mem: &mut Memory) -> bool {
        if !self.interrupts_enabled {
            return false;
        }

        let pending = mem[MappedReg::Ie] & mem[MappedReg::If];
        let bit = pending.trailing_zeros() as u16;
        if bit > 7 {
            // No interrupts are pending.
            return false;
        }
        // Toggle off the flag bit to mark the interrupt as handled.
        mem[MappedReg::If] ^= 1 << bit;
        // Disable interrupts inside the interrupt handler by default.
        self.di();

        // Bit 0: VBlank   Interrupt Request (INT $40)
        // Bit 1: LCD STAT Interrupt Request (INT $48)
        // Bit 2: Timer    Interrupt Request (INT $50)
        // Bit 3: Serial   Interrupt Request (INT $58)
        // Bit 4: Joypad   Interrupt Request (INT $60)
        let addr = 0x40 + bit * 0x8;

        self.call_addr(addr, mem);

        self.cycles_remaining = 5;
        true
    }
}
