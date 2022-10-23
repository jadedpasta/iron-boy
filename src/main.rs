use std::{env, fs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reg8(u8);

#[allow(dead_code)]
impl Reg8 {
    const C: Self = Self(0);
    const B: Self = Self(1);
    const E: Self = Self(2);
    const D: Self = Self(3);
    const L: Self = Self(4);
    const H: Self = Self(5);
    const F: Self = Self(6);
    const A: Self = Self(7);
}

impl Reg8 {
    fn index(&self) -> usize {
        self.0 as usize
    }

    fn from_bits(bits: u8) -> Option<Self> {
        match bits & 0x7 {
            6 => None,
            7 => Some(Self::A),
            bits => Some(Self(bits ^ 0x1)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reg16(u8);

#[allow(dead_code)]
impl Reg16 {
    const BC: Self = Self(0);
    const DE: Self = Self(2);
    const HL: Self = Self(4);
    const AF: Self = Self(6);
    const SP: Self = Self(8);
}

impl Reg16 {
    fn index(&self) -> usize {
        self.0 as usize
    }

    fn from_bits(bits: u8) -> Self {
        Self((bits & 0x3) << 1)
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
}

#[derive(Debug, Default)]
struct RegisterStore {
    regs: [u8; 10],
    // af: u16,
    // bc: u16,
    // de: u16,
    // hl: u16,
    // sp: u16,
}

impl RegisterStore {
    fn read_8(&self, reg: Reg8) -> u8 {
        self.regs[reg.index()]
    }

    fn write_8(&mut self, reg: Reg8, val: u8) {
        self.regs[reg.index()] = val;
    }

    fn read_16(&self, reg: Reg16) -> u16 {
        let i = reg.index();
        u16::from_le_bytes(self.regs[i..i + 2].try_into().unwrap())
    }

    fn write_16(&mut self, reg: Reg16, val: u16) {
        let i = reg.index();
        self.regs[i..i + 2].copy_from_slice(&val.to_le_bytes());
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

#[derive(Debug, Default)]
enum Instruction {
    // ========== 8080 instructions ==========
    #[default]
    Nop,
    LdRR(Reg8, Reg8),
    LdRM(Reg8, Reg16),
    LdMR(Reg16, Reg8),
    LdD8(Reg8, u8),
    LdhCA,
    LdhAC,
    LdhAA8(u8),
    LdiAHL,
    LddAHL,
    LdD16(Reg16, u16),
    LdhA8A(u8),
    LdiHLA,
    LddHLA,
    Push(Reg16),
    Pop(Reg16),
    Xor(Reg8),
    Cpl,
    Inc8(Reg8),
    Dec8(Reg8),
    Inc16(Reg16),
    Dec16(Reg16),
    Rla,
    Rra,
    JpA16(u16),
    Jr(i8),
    JrZ(i8),
    JrNz(i8),
    JrC(i8),
    JrNc(i8),
    CallA16(u16),
    Ret,
    RetZ,
    RetNz,
    RetC,
    RetNc,
    // ========== Z80/Gameboy instructions ==========
    Bit(u8, Reg8),
    Rl(Reg8),
    Rr(Reg8),
}

impl Instruction {
    fn cycles(&self) -> usize {
        use Instruction::*;
        match self {
            // ========== 8080 instructions ==========
            Nop => 1,
            LdRR(..) => 1,
            LdRM(..) => 2,
            LdMR(..) => 2,
            LdD8(..) => 2,
            LdhCA => 2,
            LdhAC => 2,
            LdhAA8(..) => 3,
            LdiHLA => 2,
            LddHLA => 2,
            LdD16(..) => 3,
            LdhA8A(..) => 3,
            LdiAHL => 2,
            LddAHL => 2,
            Push(..) => 4,
            Pop(..) => 3,
            Xor(..) => 1,
            Cpl => 1,
            Inc8(..) => 1,
            Dec8(..) => 1,
            Inc16(..) => 2,
            Dec16(..) => 2,
            Rla => 1,
            Rra => 1,
            JpA16(..) => 4,
            Jr(..) => 3,
            JrZ(..) => 2,
            JrNz(..) => 2,
            JrC(..) => 2,
            JrNc(..) => 2,
            CallA16(..) => 6,
            Ret => 4,
            RetZ => 2,
            RetNz => 2,
            RetC => 2,
            RetNc => 2,
            // ========== Z80/Gameboy instructions ==========
            Bit(..) => 2,
            Rl(..) => 2,
            Rr(..) => 2,
        }
    }
}

#[derive(Debug, Default)]
struct Cpu {
    regs: RegisterStore,
    pc: u16,
    instruction: Instruction,
    cycles_remaining: usize,
}

impl Cpu {
    fn fetch(&mut self, memory: &[u8]) {
        println!("PC: {:#x}", self.pc);

        let inst_mem = &memory[self.pc as usize..];

        fn u16(l: u8, h: u8) -> u16 {
            u16::from_le_bytes([l, h])
        }

        use Instruction::*;
        let (instruction, len) = match *inst_mem {
            // ========== 8080 instructions ==========
            [0x00, ..] => (Nop, 1),

            // 8-bit load (and HALT)
            [op, ..] if op & 0xc0 == 0x40 => {
                match (Reg8::from_bits(op >> 3), Reg8::from_bits(op)) {
                    (Some(dst), Some(src)) => (LdRR(dst, src), 1),
                    (Some(dst), None) => (LdRM(dst, Reg16::HL), 1),
                    (None, Some(src)) => (LdMR(Reg16::HL, src), 1),
                    _ => todo!(),
                }
            }
            // 8-bit load into A from memory
            [0x0a, ..] => (LdRM(Reg8::A, Reg16::BC), 1),
            [0x1a, ..] => (LdRM(Reg8::A, Reg16::DE), 1),
            // 8-bit load into memory from A
            [0x02, ..] => (LdRM(Reg8::A, Reg16::BC), 1),
            [0x12, ..] => (LdRM(Reg8::A, Reg16::DE), 1),

            // 8-bit load immediate
            [op, b, ..] if op & 0xc7 == 0x06 => match Reg8::from_bits(op >> 3) {
                Some(reg) => (LdD8(reg, b), 2),
                None => todo!(),
            },

            // Load/store from end of memory
            [0xe2, ..] => (LdhCA, 1),
            [0xf2, ..] => (LdhAC, 1),

            // Load from immediate 8-bit address
            [0xf0, b, ..] => (LdhAA8(b), 2),

            // 8-bit load inc/dec
            [0x2a, ..] => (LdiAHL, 1),
            [0x3a, ..] => (LddAHL, 1),

            // 16-bit load immediate
            [op, l, h, ..] if op & 0xcf == 0x01 => {
                (LdD16(Reg16::from_bits_sp(op >> 4), u16(l, h)), 3)
            }

            // Store to immediate 8-bit address
            [0xe0, b, ..] => (LdhA8A(b), 2),

            // 8-bit store inc/dec
            [0x22, ..] => (LdiHLA, 1),
            [0x32, ..] => (LddHLA, 1),

            // 16-bit push/pop
            [op, ..] if op & 0xcf == 0xc5 => (Push(Reg16::from_bits(op >> 4)), 1),
            [op, ..] if op & 0xcf == 0xc1 => (Pop(Reg16::from_bits(op >> 4)), 1),

            // Xor
            [op, ..] if op & 0xf8 == 0xa8 => match Reg8::from_bits(op) {
                Some(reg) => (Xor(reg), 1),
                None => todo!(),
            },

            // Complement
            [0x2f, ..] => (Cpl, 1),

            // 8-bit increment
            [op, ..] if op & 0xe7 == 0x04 => match Reg8::from_bits(op >> 3) {
                Some(reg) => (Inc8(reg), 1),
                None => todo!(),
            },
            // 8-bit decrement
            [op, ..] if op & 0xe7 == 0x05 => match Reg8::from_bits(op >> 3) {
                Some(reg) => (Dec8(reg), 1),
                None => todo!(),
            },
            // 16-bit increment/decrement
            [op, ..] if op & 0xcf == 0x03 => (Inc16(Reg16::from_bits_sp(op >> 3)), 1),
            [op, ..] if op & 0xcf == 0x0b => (Dec16(Reg16::from_bits_sp(op >> 3)), 1),

            // Rotate A register
            [0x17, ..] => (Rla, 1),
            [0x1f, ..] => (Rra, 1),

            // Jump to 16-bit address
            [0xc3, l, h, ..] => (JpA16(u16(l, h)), 3),

            // (Conditional) jump relative signed 8-bit address
            [0x20, b, ..] => (JrNz(b as i8), 2),
            [0x30, b, ..] => (JrNc(b as i8), 2),
            [0x18, b, ..] => (Jr(b as i8), 2),
            [0x28, b, ..] => (JrZ(b as i8), 2),
            [0x38, b, ..] => (JrC(b as i8), 2),

            // Call function at immediate 16-bit address
            [0xcd, l, h, ..] => (CallA16(u16(l, h)), 3),

            // (Conditional) return from function
            [0xc0, ..] => (RetNz, 1),
            [0xd0, ..] => (RetNc, 1),
            [0xc8, ..] => (RetZ, 1),
            [0xd8, ..] => (RetC, 1),
            [0xc9, ..] => (Ret, 1),

            // ========== Z80/Gameboy instructions ==========
            // Test bit in register
            [0xcb, op, ..] if op & 0xc0 == 0x40 => {
                let bit = (op >> 3) & 0x7;
                match Reg8::from_bits(op) {
                    Some(reg) => (Bit(bit, reg), 2),
                    None => todo!(),
                }
            }
            [0xcb, op, ..] if op & 0xf8 == 0x10 => match Reg8::from_bits(op) {
                Some(reg) => (Rl(reg), 2),
                None => todo!(),
            },
            [0xcb, op, ..] if op & 0xf8 == 0x18 => match Reg8::from_bits(op) {
                Some(reg) => (Rr(reg), 2),
                None => todo!(),
            },

            [0xcb, op, ..] => unimplemented!("CPU instruction with opcode: 0xcb {op:#x}"),
            [op, ..] => unimplemented!("CPU instruction with opcode: {op:#x}"),

            [] => panic!("Tried to fetch instruction from the end of memory"),
        };

        self.pc += len;
        self.cycles_remaining = instruction.cycles();
        self.instruction = instruction;
    }

    fn execute(&mut self, memory: &mut [u8]) {
        println!("Executing: {:?}", self.instruction);
        let regs = &mut self.regs;
        use Instruction::*;
        match self.instruction {
            // ========== 8080 instructions ==========
            Nop => (),
            LdRR(dst, src) => regs.write_8(dst, regs.read_8(src)),
            LdRM(dst, src) => regs.write_8(dst, memory[regs.read_16(src) as usize]),
            LdMR(dst, src) => memory[regs.read_16(dst) as usize] = regs.read_8(src),
            LdD8(reg, val) => regs.write_8(reg, val),
            LdhAC => regs.write_8(Reg8::A, memory[0xff00 + regs.read_8(Reg8::C) as usize]),
            LdhCA => {
                memory[0xff00 + regs.read_8(Reg8::C) as usize] = regs.read_8(Reg8::A);
            }
            LdhAA8(addr) => regs.write_8(Reg8::A, memory[0xff00 + addr as usize]),
            LdiAHL => {
                let addr = regs.read_16(Reg16::HL);
                regs.write_8(Reg8::A, memory[addr as usize]);
                regs.write_16(Reg16::HL, addr.wrapping_add(1));
            }
            LddAHL => {
                let addr = regs.read_16(Reg16::HL);
                regs.write_8(Reg8::A, memory[addr as usize]);
                regs.write_16(Reg16::HL, addr.wrapping_sub(1));
            }
            LdD16(reg, val) => regs.write_16(reg, val),
            LdhA8A(addr) => memory[0xff00 + addr as usize] = regs.read_8(Reg8::A),
            LdiHLA => {
                let addr = regs.read_16(Reg16::HL);
                memory[addr as usize] = regs.read_8(Reg8::A);
                regs.write_16(Reg16::HL, addr.wrapping_add(1));
            }
            LddHLA => {
                let addr = regs.read_16(Reg16::HL);
                memory[addr as usize] = regs.read_8(Reg8::A);
                regs.write_16(Reg16::HL, addr.wrapping_sub(1));
            }
            Push(reg) => {
                // Decrement the stack pointer
                let sp = regs.read_16(Reg16::SP).wrapping_sub(2);
                regs.write_16(Reg16::SP, sp);
                // Write the value onto the stack
                let sp = sp as usize;
                memory[sp..sp + 2].copy_from_slice(&regs.read_16(reg).to_le_bytes());
            }
            Pop(reg) => {
                // Increment the stack pointer
                let sp = regs.read_16(Reg16::SP);
                regs.write_16(Reg16::SP, sp.wrapping_add(2));
                // Write the value into the register
                let sp = sp as usize;
                regs.write_16(
                    reg,
                    u16::from_le_bytes(memory[sp..sp + 2].try_into().unwrap()),
                );
            }
            Xor(reg) => {
                let mut a = regs.read_8(Reg8::A);
                a ^= regs.read_8(reg);
                regs.write_8(Reg8::A, a);
                regs.set_flags(Flag::ALL, false);
                regs.set_flags(Flag::ZERO, a == 0);
            }
            Cpl => {
                let a = regs.read_8(Reg8::A);
                regs.write_8(Reg8::A, !a);
                regs.set_flags(Flag::SUB | Flag::HALFCARRY, true)
            }
            Inc8(reg) => {
                let mut val = regs.read_8(reg);
                regs.set_flags(Flag::HALFCARRY, (val & 0x0f) == 0x0f);
                val = val.wrapping_add(1);
                regs.write_8(reg, val);
                regs.set_flags(Flag::ZERO, val == 0);
                regs.set_flags(Flag::SUB, false);
            }
            Dec8(reg) => {
                let mut val = regs.read_8(reg);
                regs.set_flags(Flag::HALFCARRY, (val & 0x0f) != 0x00);
                val = val.wrapping_sub(1);
                regs.write_8(reg, val);
                regs.set_flags(Flag::ZERO, val == 0);
                regs.set_flags(Flag::SUB, true);
            }
            Inc16(reg) => regs.write_16(reg, regs.read_16(reg).wrapping_add(1)),
            Dec16(reg) => regs.write_16(reg, regs.read_16(reg).wrapping_sub(1)),
            Rla => {
                let mut val = regs.read_8(Reg8::A);
                let new_carry = val & 0x01;
                val >>= 1;
                val |= (regs.get_flag(Flag::CARRY) as u8) << 7;
                regs.write_8(Reg8::A, val);
                regs.set_flags(Flag::ALL, false);
                regs.set_flags(Flag::CARRY, new_carry != 0);
            }
            Rra => {
                let mut val = regs.read_8(Reg8::A);
                let new_carry = val & 0x01;
                val >>= 1;
                val |= (regs.get_flag(Flag::CARRY) as u8) << 7;
                regs.write_8(Reg8::A, val);
                regs.set_flags(Flag::ALL, false);
                regs.set_flags(Flag::CARRY, new_carry != 0);
            }
            JpA16(addr) => self.pc = addr,
            Jr(addr) => self.pc = self.pc.wrapping_add(addr as u16),
            JrZ(addr) => {
                if regs.get_flag(Flag::ZERO) {
                    self.instruction = Jr(addr);
                    self.cycles_remaining = 1;
                }
            }
            JrNz(addr) => {
                if !regs.get_flag(Flag::ZERO) {
                    self.instruction = Jr(addr);
                    self.cycles_remaining = 1;
                }
            }
            JrC(addr) => {
                if regs.get_flag(Flag::CARRY) {
                    self.instruction = Jr(addr);
                    self.cycles_remaining = 1;
                }
            }
            JrNc(addr) => {
                if !regs.get_flag(Flag::CARRY) {
                    self.instruction = Jr(addr);
                    self.cycles_remaining = 1;
                }
            }
            CallA16(addr) => {
                // Decrement the stack pointer
                let sp = regs.read_16(Reg16::SP).wrapping_sub(2);
                regs.write_16(Reg16::SP, sp);
                // Write the return address onto the stack
                let sp = sp as usize;
                memory[sp..sp + 2].copy_from_slice(&self.pc.to_le_bytes());
                // Jump to the function address
                self.pc = addr;
            }
            Ret => {
                // Read the return address from the stack and jump to it
                let sp = regs.read_16(Reg16::SP);
                let addr = sp as usize;
                self.pc = u16::from_le_bytes(memory[addr..addr + 2].try_into().unwrap());
                // Increment the stack pointer
                regs.write_16(Reg16::SP, sp.wrapping_add(2));
            }
            RetZ => {
                if regs.get_flag(Flag::ZERO) {
                    self.instruction = Ret;
                    self.cycles_remaining = 3;
                }
            }
            RetNz => {
                if !regs.get_flag(Flag::ZERO) {
                    self.instruction = Ret;
                    self.cycles_remaining = 3;
                }
            }
            RetC => {
                if regs.get_flag(Flag::CARRY) {
                    self.instruction = Ret;
                    self.cycles_remaining = 3;
                }
            }
            RetNc => {
                if !regs.get_flag(Flag::CARRY) {
                    self.instruction = Ret;
                    self.cycles_remaining = 3;
                }
            }
            // ========== Z80/Gameboy instructions ==========
            Bit(bit, reg) => {
                regs.set_flags(Flag::ZERO, regs.read_8(reg) & (1 << bit) == 0);
                regs.set_flags(Flag::SUB, false);
                regs.set_flags(Flag::HALFCARRY, true);
            }
            Rl(reg) => {
                let mut val = regs.read_8(reg);
                let new_carry = val >> 7;
                val <<= 1;
                val |= regs.get_flag(Flag::CARRY) as u8;
                regs.write_8(reg, val);
                regs.set_flags(Flag::ALL, false);
                regs.set_flags(Flag::ZERO, val == 0);
                regs.set_flags(Flag::CARRY, new_carry != 0);
            }
            Rr(reg) => {
                let mut val = regs.read_8(reg);
                let new_carry = val & 0x01;
                val >>= 1;
                val |= (regs.get_flag(Flag::CARRY) as u8) << 7;
                regs.write_8(reg, val);
                regs.set_flags(Flag::ALL, false);
                regs.set_flags(Flag::ZERO, val == 0);
                regs.set_flags(Flag::CARRY, new_carry != 0);
            }
        }
    }

    fn cycle(&mut self, memory: &mut [u8]) {
        if self.cycles_remaining == 0 {
            self.fetch(memory);
        }
        self.cycles_remaining -= 1;
        if self.cycles_remaining == 0 {
            self.execute(memory);
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut memory = fs::read(&args[1]).unwrap();
    memory.resize(0x10000, 0);
    let mut cpu = Cpu::default();
    loop {
        cpu.cycle(&mut memory[..]);
    }
}
