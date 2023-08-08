use super::{
    instruction_set::{HlIncDec, Operand8, Var8},
    Cpu, CpuBus, Reg16, Reg8,
};

impl Cpu {
    pub(super) fn load(&mut self, dst: Var8, src: Operand8, bus: &mut impl CpuBus) {
        let val = self.read_operand(src, bus);
        self.write_var(dst, val, bus);
    }

    pub(super) fn load_reg_mem_a(&mut self, reg: Reg16, bus: &mut impl CpuBus) {
        bus.write_8(self.regs[reg], self.regs[Reg8::A]);
    }

    pub(super) fn load_a_reg_mem(&mut self, reg: Reg16, bus: &impl CpuBus) {
        self.regs[Reg8::A] = bus.read_8(self.regs[reg]);
    }

    pub(super) fn load_imm_mem_a(&mut self, bus: &mut impl CpuBus) {
        bus.write_8(self.read_immedate_16(bus), self.regs[Reg8::A]);
    }

    pub(super) fn load_a_imm_mem(&mut self, bus: &mut impl CpuBus) {
        let addr = self.read_immedate_16(bus);
        self.regs[Reg8::A] = bus.read_8(addr);
    }

    pub(super) fn load_high_imm_mem_a(&mut self, bus: &mut impl CpuBus) {
        let addr = 0xff00 | (self.read_immedate_8(bus) as u16);
        bus.write_8(addr, self.regs[Reg8::A]);
    }

    pub(super) fn load_high_a_imm_mem(&mut self, bus: &impl CpuBus) {
        let addr = 0xff00 | (self.read_immedate_8(bus) as u16);
        self.regs[Reg8::A] = bus.read_8(addr);
    }

    pub(super) fn load_high_c_mem_a(&mut self, bus: &mut impl CpuBus) {
        let addr = 0xff00 | self.regs[Reg8::C] as u16;
        bus.write_8(addr, self.regs[Reg8::A]);
    }

    pub(super) fn load_high_a_c_mem(&mut self, bus: &impl CpuBus) {
        let addr = 0xff00 | self.regs[Reg8::C] as u16;
        self.regs[Reg8::A] = bus.read_8(addr);
    }

    fn inc_dec(&mut self, inc_dec: HlIncDec) {
        let hl = &mut self.regs[Reg16::HL];
        *hl = match inc_dec {
            HlIncDec::Inc => hl.wrapping_add(1),
            HlIncDec::Dec => hl.wrapping_sub(1),
        };
    }

    pub(super) fn load_inc_dec_a(&mut self, inc_dec: HlIncDec, bus: &mut impl CpuBus) {
        bus.write_8(self.regs[Reg16::HL], self.regs[Reg8::A]);
        self.inc_dec(inc_dec);
    }

    pub(super) fn load_a_inc_dec(&mut self, inc_dec: HlIncDec, bus: &mut impl CpuBus) {
        self.regs[Reg8::A] = bus.read_8(self.regs[Reg16::HL]);
        self.inc_dec(inc_dec);
    }

    pub(super) fn load_16(&mut self, reg: Reg16, bus: &impl CpuBus) {
        self.regs[reg] = self.read_immedate_16(bus);
    }

    pub(super) fn load_imm_mem_sp(&mut self, bus: &mut impl CpuBus) {
        bus.write_16(self.read_immedate_16(bus), self.regs[Reg16::SP]);
    }

    pub(super) fn load_sp_hl(&mut self) {
        self.regs[Reg16::SP] = self.regs[Reg16::HL];
    }

    pub(super) fn push(&mut self, reg: Reg16, bus: &mut impl CpuBus) {
        let sp = &mut self.regs[Reg16::SP];
        *sp = sp.wrapping_sub(2);
        bus.write_16(*sp, self.regs[reg]);
    }

    pub(super) fn pop(&mut self, reg: Reg16, bus: &impl CpuBus) {
        let sp = &mut self.regs[Reg16::SP];
        let val = bus.read_16(*sp);
        *sp = sp.wrapping_add(2);
        self.regs[reg] = val;
    }
}
