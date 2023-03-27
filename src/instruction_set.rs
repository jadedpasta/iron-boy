use crate::{Reg16, Reg8};

// Operands can be:
//
// Imm8
// Imm16
// Reg8
// Reg16
// (Imm8)
// (Imm16)
// (Reg8)
// (Reg16)
// (HL+)
// (HL-)

// Add SP, Imm8

// Add16 Reg16, Reg16

// Dec16 Reg16

// Inc16 Reg16

// Ld16 Reg16, Imm16

// Ld16 (Imm16), SP
// Ld16 HL, SP+Imm8
// Ld16 SP, HL

// Pop Reg16
// Push Reg16

// Bit Bit, Reg8
// Bit Bit, (Reg16)
//
// Dec Reg8
// Dec (Reg16)
//
// Inc Reg8
// Inc (Reg16)
//
// Res Bit, Reg8
// Res Bit, (Reg16)
//
// Rl Reg8
// Rl (Reg16)
//
// Rlc Reg8
// Rlc (Reg16)
//
// Rr Reg8
// Rr (Reg16)
//
// Rrc Reg8
// Rrc (Reg16)
//
// Set Bit, Reg8
// Set Bit, (Reg16)
//
// Sla Reg8
// Sla (Reg16)
//
// Sra Reg8
// Sra (Reg16)
//
// Srl Reg8
// Srl (Reg16)
//
// Swap Reg8
// Swap (Reg16)
#[derive(Copy, Clone)]
pub enum Var8 {
    Reg(Reg8),
    Mem(Reg16),
}

impl Var8 {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0x7 {
            6 => Self::Mem(Reg16::HL),
            7 => Self::Reg(Reg8::A),
            bits => Self::Reg(Reg8::from_bits(bits)),
        }
    }
}

// Adc A, Imm8
// Adc A, Reg8
// Adc A, (Reg16)
//
// Add A, Imm8
// Add A, Reg8
// Add A, (Reg16)
//
// And Imm8
// And Reg8
// And (Reg16)
//
// Cp Imm8
// Cp Reg8
// Cp (Reg16)
//
// Or Reg8
// Or Imm8
// Or (Reg16)
//
// Sbc A, Reg8
// Sbc A, Imm8
// Sbc A, (Reg16)
//
// Sub Imm8
// Sub Reg8
// Sub (Reg16)
//
// Xor Imm8
// Xor Reg8
// Xor (Reg16)
pub enum Operand8 {
    Imm,
    Var(Var8),
}

impl Operand8 {
    const fn new_reg(reg: Reg8) -> Self {
        Self::Var(Var8::Reg(reg))
    }

    const fn new_mem(reg: Reg16) -> Self {
        Self::Var(Var8::Mem(reg))
    }
}

// Call Flag, Imm16
// Call Imm16
pub enum Test {
    C,
    Z,
    Nc,
    Nz,
}

// Jp Flag, Imm16
// Jp Reg16
// Jp Imm16

// Jr Flag, Imm8
// Jr Imm8

// Ret Flag

// Ld Reg8, Imm8
// Ld Reg8, Reg8
// Ld Reg8, (Reg8)
// Ld Reg8, (Reg16)
// Ld Reg8, (Imm16)
// Ld Reg8, (Reg16-)
// Ld Reg8, (Reg16+)
// Ld (Reg16), Imm8
// Ld (Imm16), Reg8
// Ld (Reg8), Reg8
// Ld (Reg16-), Reg8
// Ld (Reg16+), Reg8
// Ld (Reg16), Reg8
pub enum LoadSpecial8 {
    MemImm16,
    MemC,
    MemHlI,
    MemHlD,
}

pub enum LoadSrc8 {
    Operand(Operand8),
    Special(LoadSpecial8),
}

impl LoadSrc8 {
    const fn new_imm() -> Self {
        Self::Operand(Operand8::Imm)
    }

    const fn new_reg(reg: Reg8) -> Self {
        Self::Operand(Operand8::new_reg(reg))
    }

    const fn new_mem(reg: Reg16) -> Self {
        Self::Operand(Operand8::new_mem(reg))
    }
}

pub enum LoadDst8 {
    Var(Var8),
    Special(LoadSpecial8),
}

impl LoadDst8 {
    const fn new_reg(reg: Reg8) -> Self {
        Self::Var(Var8::Reg(reg))
    }

    const fn new_mem(reg: Reg16) -> Self {
        Self::Var(Var8::Mem(reg))
    }
}

// Ldh (Imm8), Reg8
// Ldh Reg8, (Imm8)

// Rst VecAddr

// Stop Imm8

pub enum Instruction {
    Nop,

    // Load 8
    Ld(LoadDst8, LoadSrc8),
    LdhA2Mem, // LDH (Imm16), A
    LdhMem2A, // LDH A, (Imm16)

    // Load 16
    Ld16(Reg16), // LD Reg16, Imm16
    LdSp2Mem,    // LD (Imm16), SP
    LdSp2Hl,     // LD HL, SP+Imm8
    LdHl2Sp,     // LD SP, HL
    Pop(Reg16),
    Push(Reg16),

    // ALU 8
    Bit(u8, Var8),
    Dec(Var8),
    Inc(Var8),
    Res(u8, Var8),
    Rla, // Reset ZF
    Rl(Var8),
    Rlca, // Reset ZF
    Rlc(Var8),
    Rra, // Reset ZF
    Rr(Var8),
    Rrca, // Reset ZF
    Rrc(Var8),
    Set(u8, Var8),
    Sla(Var8),
    Sra(Var8),
    Srl(Var8),
    Swap(Var8),
    Adc(Operand8),
    Add(Operand8),
    And(Operand8),
    Cp(Operand8),
    Or(Operand8),
    Sbc(Operand8),
    Sub(Operand8),
    Xor(Operand8),
    Cpl,
    Daa,

    // ALU 16
    AddHl(Reg16),
    AddSp,
    Dec16(Reg16),
    Inc16(Reg16),

    // Flag manipulation
    Ccf,
    Scf,

    // Control
    Call(Option<Test>),
    Jp(Option<Test>),
    JpHl,
    Jr(Option<Test>),
    Rst(u8),

    // Ret Flag
    Ret(Option<Test>),
    Reti,

    // Interrupts
    Di,
    Ei,
    Halt,

    Stop,

    // All unmapped opcodes
    Illegal,
}

pub struct InstructionEntry {
    pub instruction: Instruction,
    pub size: usize,
    pub cycles: usize,
    pub branch_cycles: usize,
}

const fn new(instruction: Instruction, size: usize, cycles: usize) -> InstructionEntry {
    InstructionEntry {
        instruction,
        size,
        cycles,
        branch_cycles: 0,
    }
}

use Instruction::*;
const OP_TABLE: [InstructionEntry; 0x100] = [
    new(Nop, 1, 1),                                                          // 0x00
    new(Ld16(Reg16::BC), 3, 3),                                              // 0x01
    new(Ld(LoadDst8::new_mem(Reg16::BC), LoadSrc8::new_reg(Reg8::A)), 1, 2), // 0x02
    new(Inc16(Reg16::BC), 1, 2),                                             // 0x03
    new(Inc(Var8::Reg(Reg8::B)), 1, 1),                                      // 0x04
    new(Dec(Var8::Reg(Reg8::B)), 1, 1),                                      // 0x05
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_imm()), 2, 2),          // 0x06
    new(Rlca, 1, 1),                                                         // 0x07
    new(LdSp2Mem, 3, 5),                                                     // 0x08
    new(AddHl(Reg16::BC), 1, 2),                                             // 0x09
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_mem(Reg16::BC)), 1, 2), // 0x0a
    new(Dec16(Reg16::BC), 1, 2),                                             // 0x0b
    new(Inc(Var8::Reg(Reg8::C)), 1, 1),                                      // 0x0c
    new(Dec(Var8::Reg(Reg8::C)), 1, 1),                                      // 0x0d
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_imm()), 2, 2),          // 0x0e
    new(Rrca, 1, 1),                                                         // 0x0f
    new(Stop, 2, 1),                                                         // 0x10
    new(Ld16(Reg16::DE), 3, 3),                                              // 0x11
    new(Ld(LoadDst8::new_mem(Reg16::DE), LoadSrc8::new_reg(Reg8::A)), 1, 2), // 0x12
    new(Inc16(Reg16::DE), 1, 2),                                             // 0x13
    new(Inc(Var8::Reg(Reg8::D)), 1, 1),                                      // 0x14
    new(Dec(Var8::Reg(Reg8::D)), 1, 1),                                      // 0x15
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_imm()), 2, 2),          // 0x16
    new(Rla, 1, 1),                                                          // 0x17
    new(Jr(None), 2, 3),                                                     // 0x18
    new(AddHl(Reg16::DE), 1, 2),                                             // 0x19
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_mem(Reg16::DE)), 1, 2), // 0x1a
    new(Dec16(Reg16::DE), 1, 2),                                             // 0x1b
    new(Inc(Var8::Reg(Reg8::E)), 1, 1),                                      // 0x1c
    new(Dec(Var8::Reg(Reg8::E)), 1, 1),                                      // 0x1d
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_imm()), 2, 2),          // 0x1e
    new(Rra, 1, 1),                                                          // 0x1f
    InstructionEntry {
        instruction: Jr(Some(Test::Nz)),
        size: 2,
        cycles: 3,
        branch_cycles: 2,
    }, // 0x20
    new(Ld16(Reg16::HL), 3, 3),                                              // 0x21
    new(Ld(LoadDst8::Special(LoadSpecial8::MemHlI), LoadSrc8::new_reg(Reg8::A)), 1, 2), // 0x22
    new(Inc16(Reg16::HL), 1, 2),                                             // 0x23
    new(Inc(Var8::Reg(Reg8::H)), 1, 1),                                      // 0x24
    new(Dec(Var8::Reg(Reg8::H)), 1, 1),                                      // 0x25
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_imm()), 2, 2),          // 0x26
    new(Daa, 1, 1),                                                          // 0x27
    InstructionEntry {
        instruction: Jr(Some(Test::Z)),
        size: 2,
        cycles: 3,
        branch_cycles: 2,
    }, // 0x28
    new(AddHl(Reg16::HL), 1, 2),                                             // 0x29
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::Special(LoadSpecial8::MemHlI)), 1, 2), // 0x2a
    new(Dec16(Reg16::HL), 1, 2),                                             // 0x2b
    new(Inc(Var8::Reg(Reg8::L)), 1, 1),                                      // 0x2c
    new(Dec(Var8::Reg(Reg8::L)), 1, 1),                                      // 0x2d
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_imm()), 2, 2),          // 0x2e
    new(Cpl, 1, 1),                                                          // 0x2f
    InstructionEntry {
        instruction: Jr(Some(Test::Nc)),
        size: 2,
        cycles: 3,
        branch_cycles: 2,
    }, // 0x30
    new(Ld16(Reg16::SP), 3, 3),                                              // 0x31
    new(Ld(LoadDst8::Special(LoadSpecial8::MemHlD), LoadSrc8::new_reg(Reg8::A)), 1, 2), // 0x32
    new(Inc16(Reg16::SP), 1, 2),                                             // 0x33
    new(Inc(Var8::Mem(Reg16::HL)), 1, 3),                                    // 0x34
    new(Dec(Var8::Mem(Reg16::HL)), 1, 3),                                    // 0x35
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_imm()), 2, 3),        // 0x36
    new(Scf, 1, 1),                                                          // 0x37
    InstructionEntry {
        instruction: Jr(Some(Test::C)),
        size: 2,
        cycles: 3,
        branch_cycles: 2,
    }, // 0x38
    new(AddHl(Reg16::SP), 1, 2),                                             // 0x39
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::Special(LoadSpecial8::MemHlD)), 1, 2), // 0x3a
    new(Dec16(Reg16::SP), 1, 2),                                             // 0x3b
    new(Inc(Var8::Reg(Reg8::A)), 1, 1),                                      // 0x3c
    new(Dec(Var8::Reg(Reg8::A)), 1, 1),                                      // 0x3d
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_imm()), 2, 2),          // 0x3e
    new(Ccf, 1, 1),                                                          // 0x3f
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_reg(Reg8::B)), 1, 1),   // 0x40
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_reg(Reg8::C)), 1, 1),   // 0x41
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_reg(Reg8::D)), 1, 1),   // 0x42
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_reg(Reg8::E)), 1, 1),   // 0x43
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_reg(Reg8::H)), 1, 1),   // 0x44
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_reg(Reg8::L)), 1, 1),   // 0x45
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_mem(Reg16::HL)), 1, 2), // 0x46
    new(Ld(LoadDst8::new_reg(Reg8::B), LoadSrc8::new_reg(Reg8::A)), 1, 1),   // 0x47
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_reg(Reg8::B)), 1, 1),   // 0x48
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_reg(Reg8::C)), 1, 1),   // 0x49
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_reg(Reg8::D)), 1, 1),   // 0x4a
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_reg(Reg8::E)), 1, 1),   // 0x4b
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_reg(Reg8::H)), 1, 1),   // 0x4c
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_reg(Reg8::L)), 1, 1),   // 0x4d
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_mem(Reg16::HL)), 1, 2), // 0x4e
    new(Ld(LoadDst8::new_reg(Reg8::C), LoadSrc8::new_reg(Reg8::A)), 1, 1),   // 0x4f
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_reg(Reg8::B)), 1, 1),   // 0x50
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_reg(Reg8::C)), 1, 1),   // 0x51
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_reg(Reg8::D)), 1, 1),   // 0x52
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_reg(Reg8::E)), 1, 1),   // 0x53
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_reg(Reg8::H)), 1, 1),   // 0x54
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_reg(Reg8::L)), 1, 1),   // 0x55
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_mem(Reg16::HL)), 1, 2), // 0x56
    new(Ld(LoadDst8::new_reg(Reg8::D), LoadSrc8::new_reg(Reg8::A)), 1, 1),   // 0x57
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_reg(Reg8::B)), 1, 1),   // 0x58
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_reg(Reg8::C)), 1, 1),   // 0x59
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_reg(Reg8::D)), 1, 1),   // 0x5a
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_reg(Reg8::E)), 1, 1),   // 0x5b
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_reg(Reg8::H)), 1, 1),   // 0x5c
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_reg(Reg8::L)), 1, 1),   // 0x5d
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_mem(Reg16::HL)), 1, 2), // 0x5e
    new(Ld(LoadDst8::new_reg(Reg8::E), LoadSrc8::new_reg(Reg8::A)), 1, 1),   // 0x5f
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_reg(Reg8::B)), 1, 1),   // 0x60
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_reg(Reg8::C)), 1, 1),   // 0x61
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_reg(Reg8::D)), 1, 1),   // 0x62
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_reg(Reg8::E)), 1, 1),   // 0x63
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_reg(Reg8::H)), 1, 1),   // 0x64
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_reg(Reg8::L)), 1, 1),   // 0x65
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_mem(Reg16::HL)), 1, 2), // 0x66
    new(Ld(LoadDst8::new_reg(Reg8::H), LoadSrc8::new_reg(Reg8::A)), 1, 1),   // 0x67
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_reg(Reg8::B)), 1, 1),   // 0x68
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_reg(Reg8::C)), 1, 1),   // 0x69
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_reg(Reg8::D)), 1, 1),   // 0x6a
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_reg(Reg8::E)), 1, 1),   // 0x6b
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_reg(Reg8::H)), 1, 1),   // 0x6c
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_reg(Reg8::L)), 1, 1),   // 0x6d
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_mem(Reg16::HL)), 1, 2), // 0x6e
    new(Ld(LoadDst8::new_reg(Reg8::L), LoadSrc8::new_reg(Reg8::A)), 1, 1),   // 0x6f
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_reg(Reg8::B)), 1, 2), // 0x70
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_reg(Reg8::C)), 1, 2), // 0x71
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_reg(Reg8::D)), 1, 2), // 0x72
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_reg(Reg8::E)), 1, 2), // 0x73
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_reg(Reg8::H)), 1, 2), // 0x74
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_reg(Reg8::L)), 1, 2), // 0x75
    new(Halt, 1, 1),                                                         // 0x76
    new(Ld(LoadDst8::new_mem(Reg16::HL), LoadSrc8::new_reg(Reg8::A)), 1, 2), // 0x77
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_reg(Reg8::B)), 1, 1),   // 0x78
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_reg(Reg8::C)), 1, 1),   // 0x79
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_reg(Reg8::D)), 1, 1),   // 0x7a
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_reg(Reg8::E)), 1, 1),   // 0x7b
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_reg(Reg8::H)), 1, 1),   // 0x7c
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_reg(Reg8::L)), 1, 1),   // 0x7d
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_mem(Reg16::HL)), 1, 2), // 0x7e
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::new_reg(Reg8::A)), 1, 1),   // 0x7f
    new(Add(Operand8::new_reg(Reg8::B)), 1, 1),                              // 0x80
    new(Add(Operand8::new_reg(Reg8::C)), 1, 1),                              // 0x81
    new(Add(Operand8::new_reg(Reg8::D)), 1, 1),                              // 0x82
    new(Add(Operand8::new_reg(Reg8::E)), 1, 1),                              // 0x83
    new(Add(Operand8::new_reg(Reg8::H)), 1, 1),                              // 0x84
    new(Add(Operand8::new_reg(Reg8::L)), 1, 1),                              // 0x85
    new(Add(Operand8::new_mem(Reg16::HL)), 1, 2),                            // 0x86
    new(Add(Operand8::new_reg(Reg8::A)), 1, 1),                              // 0x87
    new(Adc(Operand8::new_reg(Reg8::B)), 1, 1),                              // 0x88
    new(Adc(Operand8::new_reg(Reg8::C)), 1, 1),                              // 0x89
    new(Adc(Operand8::new_reg(Reg8::D)), 1, 1),                              // 0x8a
    new(Adc(Operand8::new_reg(Reg8::E)), 1, 1),                              // 0x8b
    new(Adc(Operand8::new_reg(Reg8::H)), 1, 1),                              // 0x8c
    new(Adc(Operand8::new_reg(Reg8::L)), 1, 1),                              // 0x8d
    new(Adc(Operand8::new_mem(Reg16::HL)), 1, 2),                            // 0x8e
    new(Adc(Operand8::new_reg(Reg8::A)), 1, 1),                              // 0x8f
    new(Sub(Operand8::new_reg(Reg8::B)), 1, 1),                              // 0x90
    new(Sub(Operand8::new_reg(Reg8::C)), 1, 1),                              // 0x91
    new(Sub(Operand8::new_reg(Reg8::D)), 1, 1),                              // 0x92
    new(Sub(Operand8::new_reg(Reg8::E)), 1, 1),                              // 0x93
    new(Sub(Operand8::new_reg(Reg8::H)), 1, 1),                              // 0x94
    new(Sub(Operand8::new_reg(Reg8::L)), 1, 1),                              // 0x95
    new(Sub(Operand8::new_mem(Reg16::HL)), 1, 2),                            // 0x96
    new(Sub(Operand8::new_reg(Reg8::A)), 1, 1),                              // 0x97
    new(Sbc(Operand8::new_reg(Reg8::B)), 1, 1),                              // 0x98
    new(Sbc(Operand8::new_reg(Reg8::C)), 1, 1),                              // 0x99
    new(Sbc(Operand8::new_reg(Reg8::D)), 1, 1),                              // 0x9a
    new(Sbc(Operand8::new_reg(Reg8::E)), 1, 1),                              // 0x9b
    new(Sbc(Operand8::new_reg(Reg8::H)), 1, 1),                              // 0x9c
    new(Sbc(Operand8::new_reg(Reg8::L)), 1, 1),                              // 0x9d
    new(Sbc(Operand8::new_mem(Reg16::HL)), 1, 2),                            // 0x9e
    new(Sbc(Operand8::new_reg(Reg8::A)), 1, 1),                              // 0x9f
    new(And(Operand8::new_reg(Reg8::B)), 1, 1),                              // 0xa0
    new(And(Operand8::new_reg(Reg8::C)), 1, 1),                              // 0xa1
    new(And(Operand8::new_reg(Reg8::D)), 1, 1),                              // 0xa2
    new(And(Operand8::new_reg(Reg8::E)), 1, 1),                              // 0xa3
    new(And(Operand8::new_reg(Reg8::H)), 1, 1),                              // 0xa4
    new(And(Operand8::new_reg(Reg8::L)), 1, 1),                              // 0xa5
    new(And(Operand8::new_mem(Reg16::HL)), 1, 2),                            // 0xa6
    new(And(Operand8::new_reg(Reg8::A)), 1, 1),                              // 0xa7
    new(Xor(Operand8::new_reg(Reg8::B)), 1, 1),                              // 0xa8
    new(Xor(Operand8::new_reg(Reg8::C)), 1, 1),                              // 0xa9
    new(Xor(Operand8::new_reg(Reg8::D)), 1, 1),                              // 0xaa
    new(Xor(Operand8::new_reg(Reg8::E)), 1, 1),                              // 0xab
    new(Xor(Operand8::new_reg(Reg8::H)), 1, 1),                              // 0xac
    new(Xor(Operand8::new_reg(Reg8::L)), 1, 1),                              // 0xad
    new(Xor(Operand8::new_mem(Reg16::HL)), 1, 2),                            // 0xae
    new(Xor(Operand8::new_reg(Reg8::A)), 1, 1),                              // 0xaf
    new(Or(Operand8::new_reg(Reg8::B)), 1, 1),                               // 0xb0
    new(Or(Operand8::new_reg(Reg8::C)), 1, 1),                               // 0xb1
    new(Or(Operand8::new_reg(Reg8::D)), 1, 1),                               // 0xb2
    new(Or(Operand8::new_reg(Reg8::E)), 1, 1),                               // 0xb3
    new(Or(Operand8::new_reg(Reg8::H)), 1, 1),                               // 0xb4
    new(Or(Operand8::new_reg(Reg8::L)), 1, 1),                               // 0xb5
    new(Or(Operand8::new_mem(Reg16::HL)), 1, 2),                             // 0xb6
    new(Or(Operand8::new_reg(Reg8::A)), 1, 1),                               // 0xb7
    new(Cp(Operand8::new_reg(Reg8::B)), 1, 1),                               // 0xb8
    new(Cp(Operand8::new_reg(Reg8::C)), 1, 1),                               // 0xb9
    new(Cp(Operand8::new_reg(Reg8::D)), 1, 1),                               // 0xba
    new(Cp(Operand8::new_reg(Reg8::E)), 1, 1),                               // 0xbb
    new(Cp(Operand8::new_reg(Reg8::H)), 1, 1),                               // 0xbc
    new(Cp(Operand8::new_reg(Reg8::L)), 1, 1),                               // 0xbd
    new(Cp(Operand8::new_mem(Reg16::HL)), 1, 2),                             // 0xbe
    new(Cp(Operand8::new_reg(Reg8::A)), 1, 1),                               // 0xbf
    InstructionEntry {
        instruction: Ret(Some(Test::Nz)),
        size: 1,
        cycles: 5,
        branch_cycles: 2,
    }, // 0xc0
    new(Pop(Reg16::BC), 1, 3),                                               // 0xc1
    InstructionEntry {
        instruction: Jp(Some(Test::Nz)),
        size: 3,
        cycles: 4,
        branch_cycles: 3,
    }, // 0xc2
    new(Jp(None), 3, 4),                                                     // 0xc3
    InstructionEntry {
        instruction: Call(Some(Test::Nz)),
        size: 3,
        cycles: 6,
        branch_cycles: 3,
    }, // 0xc4
    new(Push(Reg16::BC), 1, 4),                                              // 0xc5
    new(Add(Operand8::Imm), 2, 2),                                           // 0xc6
    new(Rst(0x00), 1, 4),                                                    // 0xc7
    InstructionEntry {
        instruction: Ret(Some(Test::Z)),
        size: 1,
        cycles: 5,
        branch_cycles: 2,
    }, // 0xc8
    new(Ret(None), 1, 4),                                                    // 0xc9
    InstructionEntry {
        instruction: Jp(Some(Test::Z)),
        size: 3,
        cycles: 4,
        branch_cycles: 3,
    }, // 0xca
    new(Illegal, 1, 1),                                                      // 0xcb
    InstructionEntry {
        instruction: Call(Some(Test::Z)),
        size: 3,
        cycles: 6,
        branch_cycles: 3,
    }, // 0xcc
    new(Call(None), 3, 6),                                                   // 0xcd
    new(Adc(Operand8::Imm), 2, 2),                                           // 0xce
    new(Rst(0x08), 1, 4),                                                    // 0xcf
    InstructionEntry {
        instruction: Ret(Some(Test::Nc)),
        size: 1,
        cycles: 5,
        branch_cycles: 2,
    }, // 0xd0
    new(Pop(Reg16::DE), 1, 3),                                               // 0xd1
    InstructionEntry {
        instruction: Jp(Some(Test::Nc)),
        size: 3,
        cycles: 4,
        branch_cycles: 3,
    }, // 0xd2
    new(Illegal, 1, 1),                                                      // 0xd3
    InstructionEntry {
        instruction: Call(Some(Test::Nc)),
        size: 3,
        cycles: 6,
        branch_cycles: 3,
    }, // 0xd4
    new(Push(Reg16::DE), 1, 4),                                              // 0xd5
    new(Sub(Operand8::Imm), 2, 2),                                           // 0xd6
    new(Rst(0x10), 1, 4),                                                    // 0xd7
    InstructionEntry {
        instruction: Ret(Some(Test::C)),
        size: 1,
        cycles: 5,
        branch_cycles: 2,
    }, // 0xd8
    new(Reti, 1, 4),                                                         // 0xd9
    InstructionEntry {
        instruction: Jp(Some(Test::C)),
        size: 3,
        cycles: 4,
        branch_cycles: 3,
    }, // 0xda
    new(Illegal, 1, 1),                                                      // 0xdb
    InstructionEntry {
        instruction: Call(Some(Test::C)),
        size: 3,
        cycles: 6,
        branch_cycles: 3,
    }, // 0xdc
    new(Illegal, 1, 1),                                                      // 0xdd
    new(Sbc(Operand8::Imm), 2, 2),                                           // 0xde
    new(Rst(0x18), 1, 4),                                                    // 0xdf
    new(LdhA2Mem, 2, 3),                                                     // 0xe0
    new(Pop(Reg16::HL), 1, 3),                                               // 0xe1
    new(Ld(LoadDst8::Special(LoadSpecial8::MemC), LoadSrc8::new_reg(Reg8::A)), 1, 2), // 0xe2
    new(Illegal, 1, 1),                                                      // 0xe3
    new(Illegal, 1, 1),                                                      // 0xe4
    new(Push(Reg16::HL), 1, 4),                                              // 0xe5
    new(And(Operand8::Imm), 2, 2),                                           // 0xe6
    new(Rst(0x20), 1, 4),                                                    // 0xe7
    new(AddSp, 2, 4),                                                        // 0xe8
    new(JpHl, 1, 1),                                                         // 0xe9
    new(Ld(LoadDst8::Special(LoadSpecial8::MemImm16), LoadSrc8::new_reg(Reg8::A)), 3, 4), // 0xea
    new(Illegal, 1, 1),                                                      // 0xeb
    new(Illegal, 1, 1),                                                      // 0xec
    new(Illegal, 1, 1),                                                      // 0xed
    new(Xor(Operand8::Imm), 2, 2),                                           // 0xee
    new(Rst(0x28), 1, 4),                                                    // 0xef
    new(LdhMem2A, 2, 3),                                                     // 0xf0
    new(Pop(Reg16::AF), 1, 1),                                               // 0xf1
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::Special(LoadSpecial8::MemC)), 1, 2), // 0xf2
    new(Di, 1, 1),                                                           // 0xf3
    new(Illegal, 1, 1),                                                      // 0xf4
    new(Push(Reg16::AF), 1, 4),                                              // 0xf5
    new(Or(Operand8::Imm), 2, 2),                                            // 0xf6
    new(Rst(0x30), 1, 4),                                                    // 0xf7
    new(LdSp2Hl, 2, 3),                                                      // 0xf8
    new(LdHl2Sp, 1, 2),                                                      // 0xf9
    new(Ld(LoadDst8::new_reg(Reg8::A), LoadSrc8::Special(LoadSpecial8::MemImm16)), 3, 4), // 0xfa
    new(Ei, 1, 1),                                                           // 0xfb
    new(Illegal, 1, 1),                                                      // 0xfc
    new(Illegal, 1, 1),                                                      // 0xfd
    new(Cp(Operand8::Imm), 2, 2),                                            // 0xfe
    new(Rst(0x38), 1, 4),                                                    // 0xff
];

pub fn entry_for_opcode(opcode: u8) -> &'static InstructionEntry {
    &OP_TABLE[opcode as usize]
}

pub fn entry_for_prefix_opcode(opcode: u8) -> InstructionEntry {
    let var = Var8::from_bits(opcode);
    // TODO: Check if this gets optimized to a jump table. If not, it should be possible to force
    // it with some more bit hackery.
    let instruction = match opcode {
        0x00..=0x07 => Rlc(var),
        0x08..=0x0f => Rrc(var),
        0x10..=0x17 => Rl(var),
        0x18..=0x1f => Rr(var),
        0x20..=0x27 => Sla(var),
        0x28..=0x2f => Sra(var),
        0x30..=0x37 => Swap(var),
        0x38..=0x3f => Srl(var),
        0x40..=0x7f => Bit((opcode >> 3) & 0x7, var),
        0x80..=0xbf => Res((opcode >> 3) & 0x7, var),
        0xc0..=0xff => Set((opcode >> 3) & 0x7, var),
    };

    let cycles = match (var, &instruction) {
        (Var8::Mem(_), Bit(..)) => 3,
        (Var8::Mem(_), _) => 4,
        _ => 2,
    };

    new(instruction, 2, cycles)
}
