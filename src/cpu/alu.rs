use super::{
    instruction_set::{Operand8, Var8},
    Cpu, CpuBus, Flag, Reg16, Reg8,
};

fn add_impl(a: &mut u8, src: u8) -> (bool, bool) {
    let carry = a.checked_add(src).is_none();
    let half_carry = (*a << 4).checked_add(src << 4).is_none();
    *a = a.wrapping_add(src);
    (carry, half_carry)
}

fn sub_impl(a: &mut u8, src: u8) -> (bool, bool) {
    let borrow = *a < src;
    let half_borrow = *a & 0xf < src & 0xf;
    *a = a.wrapping_sub(src);
    (borrow, half_borrow)
}

impl Cpu {
    fn alu(&mut self, src: Operand8, f: impl FnOnce(&mut u8, u8) -> u8, bus: &impl CpuBus) {
        let src = self.read_operand(src, bus);
        self.regs[Reg8::F] = f(&mut self.regs[Reg8::A], src);
    }

    pub(super) fn adc(&mut self, src: Operand8, bus: &impl CpuBus) {
        let curr_carry = self.regs.get_flag(Flag::CARRY);
        let f = |a: &mut u8, src: u8| -> u8 {
            let (carry, half_carry) = add_impl(a, src);
            let carry = carry || curr_carry && *a == 0xff;
            let half_carry = half_carry || curr_carry && *a & 0xf == 0xf;
            *a = a.wrapping_add(curr_carry as u8);
            Flag::zero(*a == 0) | Flag::carry(carry) | Flag::half_carry(half_carry)
        };
        self.alu(src, f, bus);
    }

    pub(super) fn add(&mut self, src: Operand8, bus: &impl CpuBus) {
        fn f(a: &mut u8, src: u8) -> u8 {
            let (carry, half_carry) = add_impl(a, src);
            Flag::zero(*a == 0) | Flag::carry(carry) | Flag::half_carry(half_carry)
        }
        self.alu(src, f, bus);
    }

    pub(super) fn sbc(&mut self, src: Operand8, bus: &impl CpuBus) {
        let curr_carry = self.regs.get_flag(Flag::CARRY);
        let f = |a: &mut u8, src: u8| -> u8 {
            let (borrow, half_borrow) = sub_impl(a, src);
            let borrow = borrow || curr_carry && *a == 0x00;
            let half_borrow = half_borrow || curr_carry && *a & 0xf == 0x0;
            *a = a.wrapping_sub(curr_carry as u8);
            Flag::zero(*a == 0) | Flag::carry(borrow) | Flag::half_carry(half_borrow) | Flag::SUB
        };
        self.alu(src, f, bus);
    }

    pub(super) fn sub(&mut self, src: Operand8, bus: &impl CpuBus) {
        fn f(a: &mut u8, src: u8) -> u8 {
            let (borrow, half_borrow) = sub_impl(a, src);
            Flag::zero(*a == 0) | Flag::carry(borrow) | Flag::half_carry(half_borrow) | Flag::SUB
        }
        self.alu(src, f, bus);
    }

    pub(super) fn cp(&mut self, src: Operand8, bus: &impl CpuBus) {
        let a = self.regs[Reg8::A];
        self.sub(src, bus);
        self.regs[Reg8::A] = a;
    }

    pub(super) fn and(&mut self, src: Operand8, bus: &impl CpuBus) {
        fn f(a: &mut u8, src: u8) -> u8 {
            *a &= src;
            Flag::zero(*a == 0) | Flag::HALF_CARRY
        }
        self.alu(src, f, bus);
    }

    pub(super) fn or(&mut self, src: Operand8, bus: &impl CpuBus) {
        fn f(a: &mut u8, src: u8) -> u8 {
            *a |= src;
            Flag::zero(*a == 0)
        }
        self.alu(src, f, bus);
    }

    pub(super) fn xor(&mut self, src: Operand8, bus: &impl CpuBus) {
        fn f(a: &mut u8, src: u8) -> u8 {
            *a ^= src;
            Flag::zero(*a == 0)
        }
        self.alu(src, f, bus);
    }

    pub(super) fn daa(&mut self) {
        let mut a = self.regs[Reg8::A];

        let subtract = self.regs.get_flag(Flag::SUB);
        let carry = self.regs.get_flag(Flag::CARRY);
        let half_carry = self.regs.get_flag(Flag::HALF_CARRY);

        if subtract {
            if carry {
                a = a.wrapping_sub(0x60);
            }
            if half_carry {
                a = a.wrapping_sub(0x06);
            }
        } else {
            if carry || a > 0x99 {
                self.regs.set_flags(Flag::CARRY, true);
                a = a.wrapping_add(0x60);
            }
            if half_carry || a & 0x0f > 0x09 {
                a = a.wrapping_add(0x06);
            }
        }
        self.regs[Reg8::A] = a;

        self.regs.set_flags(Flag::ZERO, a == 0);
        self.regs.set_flags(Flag::HALF_CARRY, false);
    }

    pub(super) fn inc(&mut self, var: Var8, bus: &mut impl CpuBus) {
        let val = self.read_var(var, bus).wrapping_add(1);
        self.write_var(var, val, bus);
        self.regs.set_flags(Flag::HALF_CARRY, val & 0xf == 0);
        self.regs.set_flags(Flag::ZERO, val == 0);
        self.regs.set_flags(Flag::SUB, false);
    }

    pub(super) fn dec(&mut self, var: Var8, bus: &mut impl CpuBus) {
        let val = self.read_var(var, bus).wrapping_sub(1);
        self.write_var(var, val, bus);
        self.regs.set_flags(Flag::HALF_CARRY, val & 0xf == 0xf);
        self.regs.set_flags(Flag::ZERO, val == 0);
        self.regs.set_flags(Flag::SUB, true);
    }

    pub(super) fn cpl(&mut self) {
        let a = &mut self.regs[Reg8::A];
        *a = !*a;
        self.regs.set_flags(Flag::SUB | Flag::HALF_CARRY, true);
    }

    pub(super) fn bit(&mut self, bit: u8, var: Var8, mem: &impl CpuBus) {
        let zero = self.read_var(var, mem) & (1 << bit) == 0;
        self.regs.set_flags(Flag::ZERO, zero);
        self.regs.set_flags(Flag::HALF_CARRY, true);
        self.regs.set_flags(Flag::SUB, false);
    }

    pub(super) fn res(&mut self, bit: u8, var: Var8, mem: &mut impl CpuBus) {
        let mut val = self.read_var(var, mem);
        val &= !(1 << bit);
        self.write_var(var, val, mem);
    }

    pub(super) fn set(&mut self, bit: u8, var: Var8, mem: &mut impl CpuBus) {
        let mut val = self.read_var(var, mem);
        val |= 1 << bit;
        self.write_var(var, val, mem);
    }

    fn alu_var(&mut self, var: Var8, f: impl FnOnce(&mut u8) -> u8, mem: &mut impl CpuBus) {
        let mut val = self.read_var(var, mem);
        self.regs[Reg8::F] = f(&mut val);
        self.write_var(var, val, mem);
    }

    pub(super) fn rl(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let carry = self.regs.get_flag(Flag::CARRY) as u8;
        let f = |var: &mut u8| {
            let carry_out = Flag::carry(*var >> 7 != 0);
            *var <<= 1;
            *var |= carry;
            Flag::zero(*var == 0) | carry_out
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn rla(&mut self, mem: &mut impl CpuBus) {
        self.rl(Var8::Reg(Reg8::A), mem);
        self.regs.set_flags(Flag::ZERO, false);
    }

    pub(super) fn rlc(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let f = |var: &mut u8| {
            *var = var.rotate_left(1);
            Flag::zero(*var == 0) | Flag::carry(*var & 0x1 != 0)
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn rlca(&mut self, mem: &mut impl CpuBus) {
        self.rlc(Var8::Reg(Reg8::A), mem);
        self.regs.set_flags(Flag::ZERO, false);
    }

    pub(super) fn rr(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let carry = self.regs.get_flag(Flag::CARRY) as u8;
        let f = |var: &mut u8| {
            let carry_out = Flag::carry(*var & 0x1 != 0);
            *var >>= 1;
            *var |= carry << 7;
            Flag::zero(*var == 0) | carry_out
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn rra(&mut self, mem: &mut impl CpuBus) {
        self.rr(Var8::Reg(Reg8::A), mem);
        self.regs.set_flags(Flag::ZERO, false);
    }

    pub(super) fn rrc(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let f = |var: &mut u8| {
            *var = var.rotate_right(1);
            Flag::zero(*var == 0) | Flag::carry(*var >> 7 != 0)
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn rrca(&mut self, mem: &mut impl CpuBus) {
        self.rrc(Var8::Reg(Reg8::A), mem);
        self.regs.set_flags(Flag::ZERO, false);
    }

    pub(super) fn sla(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let f = |var: &mut u8| {
            let carry_out = Flag::carry(*var >> 7 != 0);
            *var <<= 1;
            Flag::zero(*var == 0) | carry_out
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn sra(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let f = |var: &mut u8| {
            let carry_out = Flag::carry(*var & 0x1 != 0);
            *var = ((*var as i8) >> 1) as u8;
            Flag::zero(*var == 0) | carry_out
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn srl(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let f = |var: &mut u8| {
            let carry_out = Flag::carry(*var & 0x1 != 0);
            *var >>= 1;
            Flag::zero(*var == 0) | carry_out
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn swap(&mut self, var: Var8, mem: &mut impl CpuBus) {
        let f = |var: &mut u8| {
            *var = (*var >> 4) | (*var << 4);
            Flag::zero(*var == 0)
        };
        self.alu_var(var, f, mem);
    }

    pub(super) fn inc_16(&mut self, reg: Reg16) {
        let reg = &mut self.regs[reg];
        *reg = reg.wrapping_add(1);
    }

    pub(super) fn dec_16(&mut self, reg: Reg16) {
        let reg = &mut self.regs[reg];
        *reg = reg.wrapping_sub(1);
    }

    pub(super) fn add_hl(&mut self, reg: Reg16) {
        let val = self.regs[reg];
        let hl = &mut self.regs[Reg16::HL];
        let carry = hl.checked_add(val).is_none();
        let half_carry = (*hl << 4).checked_add(val << 4).is_none();
        *hl = hl.wrapping_add(val);
        self.regs.set_flags(Flag::SUB, false);
        self.regs.set_flags(Flag::CARRY, carry);
        self.regs.set_flags(Flag::HALF_CARRY, half_carry);
    }

    pub(super) fn ccf(&mut self) {
        self.regs.set_flags(Flag::SUB | Flag::HALF_CARRY, false);
        self.regs[Reg8::F] ^= Flag::CARRY;
    }

    pub(super) fn scf(&mut self) {
        self.regs.set_flags(Flag::SUB | Flag::HALF_CARRY, false);
        self.regs.set_flags(Flag::CARRY, true);
    }

    fn sp_imm_inc(&mut self, bus: &mut impl CpuBus) -> u16 {
        let sp = self.regs[Reg16::SP];
        let imm = self.read_immedate_8(bus);

        let [mut low, _] = sp.to_le_bytes();
        let (carry, half_carry) = add_impl(&mut low, imm);

        self.regs[Reg8::F] = Flag::carry(carry) | Flag::half_carry(half_carry);
        sp.wrapping_add_signed(imm as i8 as i16)
    }

    pub(super) fn load_hl_sp_imm_inc(&mut self, bus: &mut impl CpuBus) {
        self.regs[Reg16::HL] = self.sp_imm_inc(bus);
    }

    pub(super) fn add_sp(&mut self, bus: &mut impl CpuBus) {
        self.regs[Reg16::SP] = self.sp_imm_inc(bus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Memory([u8; 0x10000]);

    impl Memory {
        fn new(initial_data: &[u8]) -> Self {
            let mut data = [0; 0x10000];
            data[..initial_data.len()].copy_from_slice(initial_data);
            Self(data)
        }
    }

    impl CpuBus for Memory {
        fn read_8(&self, addr: u16) -> u8 {
            self.0[addr as usize]
        }

        fn write_8(&mut self, addr: u16, val: u8) {
            self.0[addr as usize] = val;
        }

        fn cpu_dma_paused(&self) -> bool {
            unimplemented!();
        }

        fn pop_interrupt(&mut self) -> Option<u8> {
            unimplemented!();
        }

        fn interrupt_pending(&mut self) -> bool {
            unimplemented!();
        }
    }

    #[test]
    fn adc() {
        let mut cpu = Cpu::default();
        let mut mem = Memory::new(&[2]);
        cpu.regs[Reg8::A] = 1;

        cpu.adc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 3);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        cpu.regs.set_flags(Flag::CARRY, true);
        cpu.adc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 4);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        cpu.regs.set_flags(Flag::CARRY, true);
        mem.write_8(2, 0xf - 4);
        cpu.adc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0x10);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        cpu.regs.set_flags(Flag::CARRY, true);
        mem.write_8(3, 0xff - 0x10);
        cpu.adc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0x0);
        assert!(cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(cpu.regs.get_flag(Flag::ZERO));
    }

    #[test]
    fn add() {
        let mut cpu = Cpu::default();
        let mut mem = Memory::new(&[2]);
        cpu.regs[Reg8::A] = 1;

        cpu.add(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 3);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        mem.write_8(1, 0xf - 3 + 1);
        cpu.add(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0x10);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        mem.write_8(2, 0xf0);
        cpu.add(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0x0);
        assert!(cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(cpu.regs.get_flag(Flag::ZERO));
    }

    #[test]
    fn sbc() {
        let mut cpu = Cpu::default();
        let mut mem = Memory::new(&[1]);
        cpu.regs[Reg8::A] = 0xff;

        cpu.regs.set_flags(Flag::CARRY, true);
        cpu.sbc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0xfd);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));
        assert!(cpu.regs.get_flag(Flag::SUB));

        cpu.regs.set_flags(Flag::CARRY, true);
        mem.write_8(1, 0x0d);
        cpu.sbc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0xef);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        cpu.regs.set_flags(Flag::CARRY, true);
        mem.write_8(2, 0xee);
        cpu.sbc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0x0);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(cpu.regs.get_flag(Flag::ZERO));

        cpu.regs.set_flags(Flag::CARRY, true);
        mem.write_8(3, 0x0);
        cpu.sbc(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0xff);
        assert!(cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));
    }

    #[test]
    fn sub() {
        let mut cpu = Cpu::default();
        let mut mem = Memory::new(&[2]);
        cpu.regs[Reg8::A] = 0xff;

        cpu.sub(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0xfd);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));
        assert!(cpu.regs.get_flag(Flag::SUB));

        mem.write_8(1, 0x0e);
        cpu.sub(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0xef);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        mem.write_8(2, 0xef);
        cpu.sub(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0x0);
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(cpu.regs.get_flag(Flag::ZERO));

        mem.write_8(3, 0x1);
        cpu.sub(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0xff);
        assert!(cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));
    }

    #[test]
    fn cp() {
        let mut cpu = Cpu::default();
        let mut mem = Memory::new(&[2]);
        cpu.regs[Reg8::A] = 0xff;

        cpu.cp(Operand8::Imm, &mem);
        cpu.regs[Reg8::A] = 0xfd;
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));
        assert!(cpu.regs.get_flag(Flag::SUB));

        mem.write_8(1, 0x0e);
        cpu.cp(Operand8::Imm, &mem);
        cpu.regs[Reg8::A] = 0xef;
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));

        mem.write_8(2, 0xef);
        cpu.cp(Operand8::Imm, &mem);
        cpu.regs[Reg8::A] = 0x0;
        assert!(!cpu.regs.get_flag(Flag::CARRY));
        assert!(!cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(cpu.regs.get_flag(Flag::ZERO));

        mem.write_8(3, 0x1);
        cpu.cp(Operand8::Imm, &mem);
        cpu.regs[Reg8::A] = 0xff;
        assert!(cpu.regs.get_flag(Flag::CARRY));
        assert!(cpu.regs.get_flag(Flag::HALF_CARRY));
        assert!(!cpu.regs.get_flag(Flag::ZERO));
    }

    #[test]
    fn xor() {
        let mut cpu = Cpu::default();
        let mem = Memory::new(&[1]);
        cpu.regs[Reg8::A] = 1;

        cpu.xor(Operand8::Imm, &mem);
        assert_eq!(cpu.regs[Reg8::A], 0);
        assert!(cpu.regs.get_flag(Flag::ZERO));
    }

    #[test]
    fn daa() {
        let mut cpu = Cpu::default();
        let mem = Memory::new(&[]);

        fn to_bcd(val: u8) -> u8 {
            (((val / 10) % 10) << 4) | (val % 10)
        }

        for (a, a_bcd) in (0..100).map(to_bcd).enumerate() {
            for (b, b_bcd) in (0..100 - a as u8).map(to_bcd).enumerate() {
                cpu.regs[Reg8::A] = a_bcd;
                cpu.regs[Reg8::B] = b_bcd;
                cpu.add(Operand8::new_reg(Reg8::B), &mem);
                assert_eq!(cpu.regs[Reg8::A], a_bcd.wrapping_add(b_bcd));
                cpu.daa();
                assert_eq!(cpu.regs[Reg8::A], to_bcd((a + b) as u8));
            }
        }

        for (a, a_bcd) in (0..100).map(to_bcd).enumerate() {
            for (b, b_bcd) in (0..a as u8).map(to_bcd).enumerate() {
                cpu.regs[Reg8::A] = a_bcd;
                cpu.regs[Reg8::B] = b_bcd;
                cpu.sub(Operand8::new_reg(Reg8::B), &mem);
                assert_eq!(cpu.regs[Reg8::A], a_bcd.wrapping_sub(b_bcd));
                cpu.daa();
                assert_eq!(cpu.regs[Reg8::A], to_bcd((a - b) as u8));
            }
        }
    }
}
