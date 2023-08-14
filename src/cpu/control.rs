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

use super::{instruction_set::Test, Cpu, CpuBus, Flag, Reg16};

impl Cpu {
    fn test(&self, test: Test) -> bool {
        match test {
            Test::C => self.regs.get_flag(Flag::CARRY),
            Test::Z => self.regs.get_flag(Flag::ZERO),
            Test::Nc => !self.regs.get_flag(Flag::CARRY),
            Test::Nz => !self.regs.get_flag(Flag::ZERO),
        }
    }

    pub(super) fn jump(&mut self, bus: &impl CpuBus) {
        self.pc = self.read_immedate_16(bus);
    }

    pub(super) fn jump_hl(&mut self) {
        self.pc = self.regs[Reg16::HL];
    }

    pub(super) fn jump_conditional(&mut self, test: Test, cycles: usize, bus: &impl CpuBus) {
        if self.test(test) {
            self.jump(bus);
        } else {
            self.cycles_remaining = cycles;
            self.read_immedate_16(bus);
        }
    }

    pub(super) fn jump_relative(&mut self, bus: &impl CpuBus) {
        let offset = self.read_immedate_8(bus) as i8;
        self.pc = self.pc.wrapping_add(offset as u16);
    }

    pub(super) fn jump_relative_conditional(
        &mut self,
        test: Test,
        cycles: usize,
        bus: &impl CpuBus,
    ) {
        if self.test(test) {
            self.jump_relative(bus);
        } else {
            self.cycles_remaining = cycles;
            self.read_immedate_8(bus);
        }
    }

    pub(super) fn call_addr(&mut self, addr: u16, bus: &mut impl CpuBus) {
        let sp = &mut self.regs[Reg16::SP];
        *sp = sp.wrapping_sub(2);
        bus.write_16(*sp, self.pc);
        self.pc = addr;
    }

    pub(super) fn call(&mut self, bus: &mut impl CpuBus) {
        let addr = self.read_immedate_16(bus);
        self.call_addr(addr, bus);
    }

    pub(super) fn call_conditional(&mut self, test: Test, cycles: usize, bus: &mut impl CpuBus) {
        if self.test(test) {
            self.call(bus);
        } else {
            self.cycles_remaining = cycles;
            self.read_immedate_16(bus);
        }
    }

    pub(super) fn rst(&mut self, addr: u8, bus: &mut impl CpuBus) {
        self.call_addr(addr as u16, bus);
    }

    pub(super) fn ret(&mut self, bus: &impl CpuBus) {
        let sp = &mut self.regs[Reg16::SP];
        self.pc = bus.read_16(*sp);
        *sp = sp.wrapping_add(2);
    }

    pub(super) fn ret_conditional(&mut self, test: Test, cycles: usize, bus: &impl CpuBus) {
        if self.test(test) {
            self.ret(bus);
        } else {
            self.cycles_remaining = cycles;
        }
    }

    const SPEED_REG_ADDR: u16 = 0xff4D;
    pub(super) fn stop(&mut self, bus: &mut impl CpuBus) {
        let _ = self.read_immedate_8(bus);
        let mut reg = bus.read_8(Self::SPEED_REG_ADDR);
        if reg & 0x1 != 0 {
            // TODO: Implement this more accurately
            reg ^= 0x81;
            bus.write_8(Self::SPEED_REG_ADDR, reg);
        } else {
            unimplemented!("STOP: low power mode");
        }
    }
}
