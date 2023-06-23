use core::fmt;
use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use self::instruction_set::{Instruction, InstructionEntry, Operand8, Var8};

mod alu;
mod control;
mod instruction_set;
mod interrupt;
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

pub trait CpuBus {
    fn read_8(&self, addr: u16) -> u8;
    fn read_16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.read_8(addr), self.read_8(addr.wrapping_add(1))])
    }

    fn write_8(&mut self, addr: u16, val: u8);
    fn write_16(&mut self, addr: u16, val: u16) {
        let [low, high] = val.to_le_bytes();
        self.write_8(addr, low);
        self.write_8(addr.wrapping_add(1), high);
    }

    fn cpu_dma_paused(&self) -> bool;
    fn pop_interrupt(&mut self) -> Option<u8>;
}

#[derive(Debug, Default)]
pub struct Cpu {
    regs: RegisterSet,
    cycles_remaining: usize,
    pc: u16,
    interrupts_enabled: bool,
    enable_interrupts_timer: usize,
}

impl Cpu {
    fn read_immedate_8(&mut self, bus: &impl CpuBus) -> u8 {
        let val = bus.read_8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        return val;
    }

    fn read_immedate_16(&mut self, bus: &impl CpuBus) -> u16 {
        let val = bus.read_16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        return val;
    }

    fn read_var(&self, var: Var8, bus: &impl CpuBus) -> u8 {
        match var {
            Var8::Reg(reg) => self.regs[reg],
            Var8::MemHl => bus.read_8(self.regs[Reg16::HL]),
        }
    }

    fn write_var(&mut self, var: Var8, val: u8, bus: &mut impl CpuBus) {
        match var {
            Var8::Reg(reg) => self.regs[reg] = val,
            Var8::MemHl => bus.write_8(self.regs[Reg16::HL], val),
        }
    }

    fn read_operand(&mut self, operand: Operand8, bus: &impl CpuBus) -> u8 {
        match operand {
            Operand8::Var(var) => self.read_var(var, bus),
            Operand8::Imm => self.read_immedate_8(bus),
        }
    }

    fn execute_instruction(&mut self, bus: &mut impl CpuBus, entry: &InstructionEntry) {
        self.cycles_remaining = entry.cycles;

        use Instruction::*;
        match entry.instruction {
            Nop => (),
            Ld(dst, src) => self.load(dst, src, bus),
            LdMemRegA(reg) => self.load_reg_mem_a(reg, bus),
            LdAMemReg(reg) => self.load_a_reg_mem(reg, bus),
            LdMem16A => self.load_imm_mem_a(bus),
            LdAMem16 => self.load_a_imm_mem(bus),
            LdhMemA => self.load_high_imm_mem_a(bus),
            LdhAMem => self.load_high_a_imm_mem(bus),
            LdhMemCA => self.load_high_c_mem_a(bus),
            LdhAMemC => self.load_high_a_c_mem(bus),
            LdIncDecA(inc_dec) => self.load_inc_dec_a(inc_dec, bus),
            LdAIncDec(inc_dec) => self.load_a_inc_dec(inc_dec, bus),
            Ld16(reg) => self.load_16(reg, bus),
            // LdMemSp => (),
            // LdHlSpInc => (),
            // LdSpHl => (),
            Pop(reg) => self.pop(reg, bus),
            Push(reg) => self.push(reg, bus),
            Bit(bit, var) => self.bit(bit, var, bus),
            Res(bit, var) => self.res(bit, var, bus),
            Set(bit, var) => self.set(bit, var, bus),
            Rla => self.rla(bus),
            Rl(var) => self.rl(var, bus),
            Rlca => self.rlca(bus),
            Rlc(var) => self.rlc(var, bus),
            Rra => self.rra(bus),
            Rr(var) => self.rr(var, bus),
            Rrca => self.rrca(bus),
            Rrc(var) => self.rrc(var, bus),
            Sla(var) => self.sla(var, bus),
            Sra(var) => self.sra(var, bus),
            Srl(var) => self.srl(var, bus),
            Swap(var) => self.swap(var, bus),
            Adc(src) => self.adc(src, bus),
            Add(src) => self.add(src, bus),
            And(src) => self.and(src, bus),
            Cp(src) => self.cp(src, bus),
            Or(src) => self.or(src, bus),
            Sbc(src) => self.sbc(src, bus),
            Sub(src) => self.sub(src, bus),
            Xor(src) => self.xor(src, bus),
            Dec(var) => self.dec(var, bus),
            Inc(var) => self.inc(var, bus),
            Cpl => self.cpl(),
            Daa => self.daa(),
            Dec16(reg) => self.dec_16(reg),
            Inc16(reg) => self.inc_16(reg),
            AddHl(reg) => self.add_hl(reg),
            // AddSp => (),
            Ccf => self.ccf(),
            Scf => self.scf(),
            Call(None) => self.call(bus),
            Call(Some(test)) => self.call_conditional(test, entry.branch_cycles, bus),
            Jp(None) => self.jump(bus),
            Jp(Some(test)) => self.jump_conditional(test, entry.branch_cycles, bus),
            JpHl => self.jump_hl(),
            Jr(None) => self.jump_relative(bus),
            Jr(Some(test)) => self.jump_relative_conditional(test, entry.branch_cycles, bus),
            Rst(addr) => self.rst(addr, bus),
            Ret(None) => self.ret(bus),
            Ret(Some(test)) => self.ret_conditional(test, entry.branch_cycles, bus),
            Reti => self.reti(bus),
            Di => self.di(),
            Ei => self.ei(),
            // Halt => (),
            Stop => self.stop(bus),
            Illegal => panic!("Tried to execute illegal instruction"),
            inst => unimplemented!("{:?}", inst),
        }
    }

    pub fn execute(&mut self, bus: &mut impl CpuBus) {
        if bus.cpu_dma_paused() {
            return;
        }

        if self.cycles_remaining == 0 && !self.handle_interrupts(bus) {
            let start_pc = self.pc;
            let opcode = self.read_immedate_8(bus);

            print!("Executing({:04x}): {opcode:#02x} ", start_pc);

            let entry_data;
            let entry = if opcode == instruction_set::PREFIX_OPCODE {
                let opcode = self.read_immedate_8(bus);
                print!("{opcode:#02x} ");
                entry_data = instruction_set::entry_for_prefix_opcode(opcode);
                &entry_data
            } else {
                instruction_set::entry_for_opcode(opcode)
            };

            println!("{:?}", entry.instruction);

            self.execute_instruction(bus, &entry);
        }
        self.update_interrupt_timer();
        self.cycles_remaining -= 1;
    }
}
