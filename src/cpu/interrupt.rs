// This file is part of Iron Boy, a CGB emulator.
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
//
// This program is free software: you can redistribute it and/or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See
// the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this program. If
// not, see <https://www.gnu.org/licenses/>.

use super::{Cpu, CpuBus};

impl Cpu {
    pub(super) fn ei(&mut self) {
        self.enable_interrupts_timer = 2;
    }

    pub(super) fn di(&mut self) {
        self.enable_interrupts_timer = 0;
        self.interrupts_enabled = false;
    }

    pub(super) fn reti(&mut self, bus: &impl CpuBus) {
        self.interrupts_enabled = true;
        self.ret(bus);
    }

    pub(super) fn update_interrupt_timer(&mut self) {
        if self.enable_interrupts_timer > 0 {
            self.enable_interrupts_timer -= 1;
            if self.enable_interrupts_timer == 0 {
                self.interrupts_enabled = true;
            }
        }
    }

    pub(super) fn halt(&mut self) {
        // TODO: Halt bug
        self.halted = true;
    }

    pub(super) fn handle_interrupts(&mut self, bus: &mut impl CpuBus) -> bool {
        if !self.interrupts_enabled {
            if bus.interrupt_pending() {
                self.halted = false;
            }
            return false;
        }

        let Some(bit) = bus.pop_interrupt() else { return false; };
        // Disable interrupts inside the interrupt handler by default.
        self.di();

        // Unhalt the CPU if it's halted to handle the interrupt
        self.halted = false;

        // Bit 0: VBlank   Interrupt Request (INT $40)
        // Bit 1: LCD STAT Interrupt Request (INT $48)
        // Bit 2: Timer    Interrupt Request (INT $50)
        // Bit 3: Serial   Interrupt Request (INT $58)
        // Bit 4: Joypad   Interrupt Request (INT $60)
        let addr = 0x40 + bit as u16 * 0x8;

        self.call_addr(addr, bus);

        self.cycles_remaining = 5;
        true
    }
}
