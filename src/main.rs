use std::{
    env,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

mod instruction_set;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reg<T>(u8, PhantomData<T>);

type Reg8 = Reg<u8>;
type Reg16 = Reg<u16>;

#[allow(dead_code)]
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

#[allow(dead_code)]
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

    fn from_bits(bits: u8) -> Self {
        Self(bits & 0x3, PhantomData)
    }

    fn from_bits_sp(bits: u8) -> Self {
        let reg = Self::from_bits(bits);
        if reg == Self::AF {
            Self::SP
        } else {
            reg
        }
    }
}

enum Flag {}

impl Flag {
    const ZERO: u8 = 0x80;
    const SUB: u8 = 0x40;
    const HALFCARRY: u8 = 0x20;
    const CARRY: u8 = 0x10;
    const ALL: u8 = 0xf0;

    fn zero(val: bool) -> u8 {
        Self::ZERO * (val as u8)
    }
    fn sub(val: bool) -> u8 {
        Self::SUB * (val as u8)
    }
    fn half_carry(val: bool) -> u8 {
        Self::HALFCARRY * (val as u8)
    }
    fn carry(val: bool) -> u8 {
        Self::CARRY * (val as u8)
    }
}

#[derive(Debug, Default)]
struct RegisterSet {
    regs: [u16; 5],
    // af: u16,
    // bc: u16,
    // de: u16,
    // hl: u16,
    // sp: u16,
}

impl Index<Reg8> for RegisterSet {
    type Output = u8;
    fn index(&self, reg: Reg8) -> &Self::Output {
        let i = reg.index();
        // &self.regs[i / 2].to_le_bytes()[i % 2]
        // &LeSlice::from(&self.regs[i / 2]).index(i % 2)
        let reg16 = &self.regs[i / 2];
        let reg16 = unsafe { &*(reg16 as *const u16 as *const [u8; 2]) };
        &reg16[(i & 0x1) ^ (cfg!(target_endian = "big") as usize)]
    }
}

impl IndexMut<Reg8> for RegisterSet {
    fn index_mut(&mut self, reg: Reg8) -> &mut Self::Output {
        let i = reg.index();
        // &mut self.regs[i / 2].to_le_bytes()[i % 2]
        // &mut LeSliceMut::from(&mut self.regs[i / 2]).index_mut(i % 2)
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
    fn read_8(&self, reg: Reg8) -> u8 {
        self[reg]
    }

    fn write_8(&mut self, reg: Reg8, val: u8) {
        self[reg] = val;
    }

    fn read_16(&self, reg: Reg16) -> u16 {
        self.regs[reg.index()]
    }

    fn write_16(&mut self, reg: Reg16, val: u16) {
        self.regs[reg.index()] = val;
    }

    fn set_flags(&mut self, flags: u8, value: bool) {
        let mut f = self.read_8(Reg8::F);
        f &= !flags;
        f |= (value as u8) * flags;
        self.write_8(Reg8::F, f);
    }

    fn get_flag(&self, flag: u8) -> bool {
        self.read_8(Reg8::F) & flag != 0
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    dbg!(args);
}
