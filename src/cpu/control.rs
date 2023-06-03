use crate::memory::Memory;

use super::{instruction_set::Test, Cpu, Flag, Reg16};

impl Cpu {
    fn test(&self, test: Test) -> bool {
        match test {
            Test::C => self.regs.get_flag(Flag::CARRY),
            Test::Z => self.regs.get_flag(Flag::ZERO),
            Test::Nc => !self.regs.get_flag(Flag::CARRY),
            Test::Nz => !self.regs.get_flag(Flag::ZERO),
        }
    }

    pub(super) fn jump(&mut self, mem: &Memory) {
        self.pc = self.read_immedate_16(mem);
    }

    pub(super) fn jump_hl(&mut self) {
        self.pc = self.regs[Reg16::HL];
    }

    pub(super) fn jump_conditional(&mut self, test: Test, cycles: usize, mem: &Memory) {
        if self.test(test) {
            self.jump(mem);
        } else {
            self.cycles_remaining = cycles;
            self.read_immedate_16(mem);
        }
    }

    pub(super) fn jump_relative(&mut self, mem: &Memory) {
        let offset = self.read_immedate_8(mem) as i8;
        self.pc = self.pc.wrapping_add(offset as u16);
    }

    pub(super) fn jump_relative_conditional(&mut self, test: Test, cycles: usize, mem: &Memory) {
        if self.test(test) {
            self.jump_relative(mem);
        } else {
            self.cycles_remaining = cycles;
            self.read_immedate_8(mem);
        }
    }

    pub(super) fn call_addr(&mut self, addr: u16, mem: &mut Memory) {
        let sp = &mut self.regs[Reg16::SP];
        *sp = sp.wrapping_sub(2);
        mem.write_16(*sp, self.pc);
        self.pc = addr;
    }

    pub(super) fn call(&mut self, mem: &mut Memory) {
        let addr = self.read_immedate_16(mem);
        self.call_addr(addr, mem);
    }

    pub(super) fn call_conditional(&mut self, test: Test, cycles: usize, mem: &mut Memory) {
        if self.test(test) {
            self.call(mem);
        } else {
            self.cycles_remaining = cycles;
            self.read_immedate_16(mem);
        }
    }

    pub(super) fn rst(&mut self, addr: u8, mem: &mut Memory) {
        self.call_addr(addr as u16, mem);
    }

    pub(super) fn ret(&mut self, mem: &Memory) {
        let sp = &mut self.regs[Reg16::SP];
        self.pc = mem.read_16(*sp);
        *sp = sp.wrapping_add(2);
    }

    pub(super) fn ret_conditional(&mut self, test: Test, cycles: usize, mem: &Memory) {
        if self.test(test) {
            self.ret(mem);
        } else {
            self.cycles_remaining = cycles;
        }
    }

    const SPEED_REG_ADDR: u16 = 0xff4D;
    pub(super) fn stop(&mut self, mem: &mut Memory) {
        let _ = self.read_immedate_8(mem);
        let mut reg = mem.read_8(Self::SPEED_REG_ADDR);
        if reg & 0x1 != 0 {
            // TODO: Implement this more accurately
            reg ^= 0x81;
            mem.write_8(Self::SPEED_REG_ADDR, reg);
        } else {
            unimplemented!("STOP: low power mode");
        }
    }
}
