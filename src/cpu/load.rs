use crate::memory::Memory;

use super::{
    instruction_set::{HlIncDec, Operand8, Var8},
    Cpu, Reg16, Reg8,
};

impl Cpu {
    pub(super) fn load(&mut self, dst: Var8, src: Operand8, mem: &mut Memory) {
        let val = self.read_operand(src, mem);
        self.write_var(dst, val, mem);
    }

    pub(super) fn load_reg_mem_a(&mut self, reg: Reg16, mem: &mut Memory) {
        mem.write_8(self.regs[reg], self.regs[Reg8::A]);
    }

    pub(super) fn load_a_reg_mem(&mut self, reg: Reg16, mem: &Memory) {
        self.regs[Reg8::A] = mem.read_8(self.regs[reg]);
    }

    pub(super) fn load_imm_mem_a(&mut self, mem: &mut Memory) {
        mem.write_8(self.read_immedate_16(mem), self.regs[Reg8::A]);
    }

    pub(super) fn load_a_imm_mem(&mut self, mem: &mut Memory) {
        self.regs[Reg8::A] = mem.read_8(self.read_immedate_16(mem));
    }

    pub(super) fn load_high_imm_mem_a(&mut self, mem: &mut Memory) {
        let addr = 0xff00 | (self.read_immedate_8(mem) as u16);
        mem.write_8(addr, self.regs[Reg8::A]);
    }

    pub(super) fn load_high_a_imm_mem(&mut self, mem: &Memory) {
        let addr = 0xff00 | (self.read_immedate_8(mem) as u16);
        self.regs[Reg8::A] = mem.read_8(addr);
    }

    pub(super) fn load_high_c_mem_a(&mut self, mem: &mut Memory) {
        let addr = 0xff00 | self.regs[Reg8::C] as u16;
        mem.write_8(addr, self.regs[Reg8::A]);
    }

    pub(super) fn load_high_a_c_mem(&mut self, mem: &Memory) {
        let addr = 0xff00 | self.regs[Reg8::C] as u16;
        self.regs[Reg8::A] = mem.read_8(addr);
    }

    fn inc_dec(&mut self, inc_dec: HlIncDec) {
        let hl = &mut self.regs[Reg16::HL];
        *hl = match inc_dec {
            HlIncDec::Inc => hl.wrapping_add(1),
            HlIncDec::Dec => hl.wrapping_sub(1),
        };
    }

    pub(super) fn load_inc_dec_a(&mut self, inc_dec: HlIncDec, mem: &mut Memory) {
        mem.write_8(self.regs[Reg16::HL], self.regs[Reg8::A]);
        self.inc_dec(inc_dec);
    }

    pub(super) fn load_a_inc_dec(&mut self, inc_dec: HlIncDec, mem: &mut Memory) {
        self.regs[Reg8::A] = mem.read_8(self.regs[Reg16::HL]);
        self.inc_dec(inc_dec);
    }

    pub(super) fn load_16(&mut self, reg: Reg16, mem: &Memory) {
        self.regs[reg] = self.read_immedate_16(mem);
    }

    pub(super) fn push(&mut self, reg: Reg16, mem: &mut Memory) {
        let sp = &mut self.regs[Reg16::SP];
        *sp = sp.wrapping_sub(2);
        mem.write_16(*sp, self.regs[reg]);
    }

    pub(super) fn pop(&mut self, reg: Reg16, mem: &Memory) {
        let sp = &mut self.regs[Reg16::SP];
        let val = mem.read_16(*sp);
        *sp = sp.wrapping_add(2);
        self.regs[reg] = val;
    }
}
