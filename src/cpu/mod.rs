use core::fmt;
use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::memory::Memory;

use self::instruction_set::{Instruction, InstructionEntry, Operand8, Var8};

mod alu;
mod control;
mod instruction_set;
mod load;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Reg<T>(u8, PhantomData<T>);

type Reg8 = Reg<u8>;
type Reg16 = Reg<u16>;

impl Reg8 {
    const C: Self = Self(0, PhantomData);
    const B: Self = Self(1, PhantomData);
    const E: Self = Self(2, PhantomData);
    const D: Self = Self(3, PhantomData);
    const L: Self = Self(4, PhantomData);
    const H: Self = Self(5, PhantomData);
    const F: Self = Self(6, PhantomData);
    const A: Self = Self(7, PhantomData);
}

impl Reg8 {
    fn index(&self) -> usize {
        self.0 as usize
    }

    const fn from_bits(bits: u8) -> Self {
        Self((bits & 0x7) ^ 0x1, PhantomData)
    }
}

impl Debug for Reg8 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match *self {
                Self::C => "C",
                Self::B => "B",
                Self::E => "E",
                Self::D => "D",
                Self::L => "L",
                Self::H => "H",
                Self::F => "F",
                Self::A => "A",
                _ => panic!(),
            }
        )
    }
}

impl Reg16 {
    const BC: Self = Self(0, PhantomData);
    const DE: Self = Self(1, PhantomData);
    const HL: Self = Self(2, PhantomData);
    const AF: Self = Self(3, PhantomData);
    const SP: Self = Self(4, PhantomData);
}

impl Reg16 {
    fn index(&self) -> usize {
        self.0 as usize
    }
}

impl Debug for Reg16 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match *self {
                Self::BC => "BC",
                Self::DE => "DE",
                Self::HL => "HL",
                Self::AF => "AF",
                Self::SP => "SP",
                _ => panic!(),
            }
        )
    }
}

enum Flag {}

impl Flag {
    const ZERO: u8 = 0x80;
    const SUB: u8 = 0x40;
    const HALF_CARRY: u8 = 0x20;
    const CARRY: u8 = 0x10;

    fn zero(val: bool) -> u8 {
        Self::ZERO * (val as u8)
    }
    fn half_carry(val: bool) -> u8 {
        Self::HALF_CARRY * (val as u8)
    }
    fn carry(val: bool) -> u8 {
        Self::CARRY * (val as u8)
    }
}

#[derive(Debug, Default)]
struct RegisterSet {
    regs: [u16; 5],
    // bc: u16,
    // de: u16,
    // hl: u16,
    // sp: u16,
    // af: u16,
}

impl Index<Reg8> for RegisterSet {
    type Output = u8;
    fn index(&self, reg: Reg8) -> &Self::Output {
        let i = reg.index();
        let reg16 = &self.regs[i / 2];
        let reg16 = unsafe { &*(reg16 as *const u16 as *const [u8; 2]) };
        &reg16[(i & 0x1) ^ (cfg!(target_endian = "big") as usize)]
    }
}

impl IndexMut<Reg8> for RegisterSet {
    fn index_mut(&mut self, reg: Reg8) -> &mut Self::Output {
        let i = reg.index();
        let reg16 = &mut self.regs[i / 2];
        let reg16 = unsafe { &mut *(reg16 as *mut u16 as *mut [u8; 2]) };
        &mut reg16[(i & 0x1) ^ (cfg!(target_endian = "big") as usize)]
    }
}

impl Index<Reg16> for RegisterSet {
    type Output = u16;
    fn index(&self, reg: Reg16) -> &Self::Output {
        &self.regs[reg.index()]
    }
}

impl IndexMut<Reg16> for RegisterSet {
    fn index_mut(&mut self, reg: Reg16) -> &mut Self::Output {
        &mut self.regs[reg.index()]
    }
}

impl RegisterSet {
    fn set_flags(&mut self, flags: u8, value: bool) {
        let mut f = self[Reg8::F];
        f &= !flags;
        f |= (value as u8) * flags;
        self[Reg8::F] = f;
    }

    fn get_flag(&self, flag: u8) -> bool {
        self[Reg8::F] & flag != 0
    }
}

#[derive(Debug, Default)]
pub struct Cpu {
    regs: RegisterSet,
    cycles_remaining: usize,
    pc: u16,
}

impl Cpu {
    fn read_immedate_8(&mut self, mem: &Memory) -> u8 {
        let val = mem.read_8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        return val;
    }

    fn read_immedate_16(&mut self, mem: &Memory) -> u16 {
        let val = mem.read_16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        return val;
    }

    fn read_var(&self, var: Var8, mem: &Memory) -> u8 {
        match var {
            Var8::Reg(reg) => self.regs[reg],
            Var8::MemHl => mem.read_8(self.regs[Reg16::HL]),
        }
    }

    fn write_var(&mut self, var: Var8, val: u8, mem: &mut Memory) {
        match var {
            Var8::Reg(reg) => self.regs[reg] = val,
            Var8::MemHl => mem.write_8(self.regs[Reg16::HL], val),
        }
    }

    fn read_operand(&mut self, operand: Operand8, mem: &Memory) -> u8 {
        match operand {
            Operand8::Var(var) => self.read_var(var, mem),
            Operand8::Imm => self.read_immedate_8(mem),
        }
    }

    fn execute_instruction(&mut self, mem: &mut Memory, entry: &InstructionEntry) {
        self.cycles_remaining = entry.cycles;

        use Instruction::*;
        match entry.instruction {
            Nop => (),
            Ld(dst, src) => self.load(dst, src, mem),
            LdMemRegA(reg) => self.load_reg_mem_a(reg, mem),
            LdAMemReg(reg) => self.load_a_reg_mem(reg, mem),
            LdMem16A => self.load_imm_mem_a(mem),
            LdAMem16 => self.load_a_imm_mem(mem),
            LdhMemA => self.load_high_imm_mem_a(mem),
            LdhAMem => self.load_high_a_imm_mem(mem),
            LdhMemCA => self.load_high_c_mem_a(mem),
            LdhAMemC => self.load_high_a_c_mem(mem),
            LdIncDecA(inc_dec) => self.load_inc_dec_a(inc_dec, mem),
            LdAIncDec(inc_dec) => self.load_a_inc_dec(inc_dec, mem),
            Ld16(reg) => self.load_16(reg, mem),
            // LdMemSp => (),
            // LdHlSpInc => (),
            // LdSpHl => (),
            Pop(reg) => self.pop(reg, mem),
            Push(reg) => self.push(reg, mem),
            Bit(bit, var) => self.bit(bit, var, mem),
            Res(bit, var) => self.res(bit, var, mem),
            Set(bit, var) => self.set(bit, var, mem),
            Rla => self.rla(mem),
            Rl(var) => self.rl(var, mem),
            Rlca => self.rlca(mem),
            Rlc(var) => self.rlc(var, mem),
            Rra => self.rra(mem),
            Rr(var) => self.rr(var, mem),
            Rrca => self.rrca(mem),
            Rrc(var) => self.rrc(var, mem),
            Sla(var) => self.sla(var, mem),
            Sra(var) => self.sra(var, mem),
            Srl(var) => self.srl(var, mem),
            Swap(var) => self.swap(var, mem),
            Adc(src) => self.adc(src, mem),
            Add(src) => self.add(src, mem),
            And(src) => self.and(src, mem),
            Cp(src) => self.cp(src, mem),
            Or(src) => self.or(src, mem),
            Sbc(src) => self.sbc(src, mem),
            Sub(src) => self.sub(src, mem),
            Xor(src) => self.xor(src, mem),
            Dec(var) => self.dec(var, mem),
            Inc(var) => self.inc(var, mem),
            Cpl => self.cpl(),
            Daa => self.daa(),
            Dec16(reg) => self.inc_16(reg),
            Inc16(reg) => self.dec_16(reg),
            AddHl(reg) => self.add_hl(reg),
            // AddSp => (),
            // Ccf => (),
            // Scf => (),
            Call(None) => self.call(mem),
            Call(Some(test)) => self.call_conditional(test, entry.branch_cycles, mem),
            Jp(None) => self.jump(mem),
            Jp(Some(test)) => self.jump_conditional(test, entry.branch_cycles, mem),
            // JpHl => (),
            Jr(None) => self.jump_relative(mem),
            Jr(Some(test)) => self.jump_relative_conditional(test, entry.branch_cycles, mem),
            // Rst(u8) => (),
            Ret(None) => self.ret(mem),
            Ret(Some(test)) => self.ret_conditional(test, entry.branch_cycles, mem),
            // Reti => (),
            // Di => (),
            // Ei => (),
            // Halt => (),
            Stop => self.stop(mem),
            Illegal => panic!("Tried to execute illegal instruction"),
            inst => unimplemented!("{:?}", inst),
        }
    }

    pub fn execute(&mut self, mem: &mut Memory) {
        if self.cycles_remaining == 0 {
            let start_pc = self.pc;
            let opcode = self.read_immedate_8(mem);

            print!("Executing({:04x}): {opcode:#02x} ", start_pc);

            let entry_data;
            let entry = if opcode == instruction_set::PREFIX_OPCODE {
                let opcode = self.read_immedate_8(mem);
                print!("{opcode:#02x} ");
                entry_data = instruction_set::entry_for_prefix_opcode(opcode);
                &entry_data
            } else {
                instruction_set::entry_for_opcode(opcode)
            };

            println!("{:?}", entry.instruction);

            self.execute_instruction(mem, &entry);
        }
        self.cycles_remaining -= 1;
    }
}
