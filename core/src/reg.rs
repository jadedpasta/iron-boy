// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
#![allow(unused)]

macro_rules! u8_consts {
    ($($name:ident = $val:expr),* $(,)?) => {
        $(
            pub const $name: u8 = $val;
        )*
    };
}

u8_consts! {
    P1 = 0x00,    // Joypad                                    | Mixed | All
    SB = 0x01,    // Serial transfer data                      | R/W   | All
    SC = 0x02,    // Serial transfer control                   | R/W   | Mixed
    DIV = 0x04,   // Divider register                          | R/W   | All
    TIMA = 0x05,  // Timer counter                             | R/W   | All
    TMA = 0x06,   // Timer modulo                              | R/W   | All
    TAC = 0x07,   // Timer control                             | R/W   | All
    IF = 0x0f,    // Interrupt flag                            | R/W   | All
    NR10 = 0x10,  // Sound channel 1 sweep                     | R/W   | All
    NR11 = 0x11,  // Sound channel 1 length timer & duty cycle | Mixed | All
    NR12 = 0x12,  // Sound channel 1 volume & envelope         | R/W   | All
    NR13 = 0x13,  // Sound channel 1 wavelength low            | W     | All
    NR14 = 0x14,  // Sound channel 1 wavelength high & control | Mixed | All
    NR21 = 0x16,  // Sound channel 2 length timer & duty cycle | Mixed | All
    NR22 = 0x17,  // Sound channel 2 volume & envelope         | R/W   | All
    NR23 = 0x18,  // Sound channel 2 wavelength low            | W     | All
    NR24 = 0x19,  // Sound channel 2 wavelength high & control | Mixed | All
    NR30 = 0x1a,  // Sound channel 3 DAC enable                | R/W   | All
    NR31 = 0x1b,  // Sound channel 3 length timer              | W     | All
    NR32 = 0x1c,  // Sound channel 3 output level              | R/W   | All
    NR33 = 0x1d,  // Sound channel 3 wavelength low            | W     | All
    NR34 = 0x1e,  // Sound channel 3 wavelength high & control | Mixed | All
    NR41 = 0x20,  // Sound channel 4 length timer              | W     | All
    NR42 = 0x21,  // Sound channel 4 volume & envelope         | R/W   | All
    NR43 = 0x22,  // Sound channel 4 frequency & randomness    | R/W   | All
    NR44 = 0x23,  // Sound channel 4 control                   | Mixed | All
    NR50 = 0x24,  // Master volume & VIN panning               | R/W   | All
    NR51 = 0x25,  // Sound panning                             | R/W   | All
    NR52 = 0x26,  // Sound on/off                              | Mixed | All
    LCDC = 0x40,  // LCD control                               | R/W   | All
    STAT = 0x41,  // LCD status                                | Mixed | All
    SCY = 0x42,   // Viewport Y position                       | R/W   | All
    SCX = 0x43,   // Viewport X position                       | R/W   | All
    LY = 0x44,    // LCD Y coordinate                          | R     | All
    LYC = 0x45,   // LY compare                                | R/W   | All
    DMA = 0x46,   // OAM DMA source address & start            | R/W   | All
    BGP = 0x47,   // BG palette data                           | R/W   | DMG
    OBP0 = 0x48,  // OBJ palette 0 data                        | R/W   | DMG
    OBP1 = 0x49,  // OBJ palette 1 data                        | R/W   | DMG
    WY = 0x4a,    // Window Y position                         | R/W   | All
    WX = 0x4b,    // Window X position plus 7                  | R/W   | All
    KEY0 = 0x4c,  // Disable CGB mode; enable compat           | Mixed | CGB
    KEY1 = 0x4d,  // Prepare speed switch                      | Mixed | CGB
    VBK = 0x4f,   // VRAM bank                                 | R/W   | CGB
    BANK = 0x50,  // Write to unmap boot ROM                   | ?     | All
    HDMA1 = 0x51, // VRAM DMA source high                      | W     | CGB
    HDMA2 = 0x52, // VRAM DMA source low                       | W     | CGB
    HDMA3 = 0x53, // VRAM DMA destination high                 | W     | CGB
    HDMA4 = 0x54, // VRAM DMA destination low                  | W     | CGB
    HDMA5 = 0x55, // VRAM DMA length/mode/start                | R/W   | CGB
    RP = 0x56,    // Infrared communications port              | Mixed | CGB
    BCPS = 0x68,  // Background color palette specification    | R/W   | CGB
    BCPD = 0x69,  // Background color palette data             | R/W   | CGB
    OCPS = 0x6a,  // OBJ color palette specification           | R/W   | CGB
    OCPD = 0x6b,  // OBJ color palette data                    | R/W   | CGB
    OPRI = 0x6c,  // Object priority mode                      | R/W   | CGB
    SVBK = 0x70,  // WRAM bank                                 | R/W   | CGB
    PCM12 = 0x76, // Audio digital outputs 1 & 2               | R     | CGB
    PCM34 = 0x77, // Audio digital outputs 3 & 4               | R     | CGB
    IE = 0xff,    // Interrupt enable                          | R/W   | All
}
