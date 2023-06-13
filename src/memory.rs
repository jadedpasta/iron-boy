use std::{
    mem::{self, MaybeUninit},
    ops::{Index, IndexMut},
};

use crate::{
    dma::{DmaState, DmaType},
    joypad::{Button, ButtonState, JoypadState},
};

const BOOT_ROM: &'static [u8] = include_bytes!("../sameboy_boot.bin");
const NON_CGB_KEY0_VAL: u8 = 0x04;

#[allow(unused)]
pub enum MappedReg {
    P1 = 0xff00,    // Joypad                                    | Mixed | All
    Sb = 0xff01,    // Serial transfer data                      | R/W   | All
    Sc = 0xff02,    // Serial transfer control                   | R/W   | Mixed
    Div = 0xff04,   // Divider register                          | R/W   | All
    Tima = 0xff05,  // Timer counter                             | R/W   | All
    Tma = 0xff06,   // Timer modulo                              | R/W   | All
    Tac = 0xff07,   // Timer control                             | R/W   | All
    If = 0xff0f,    // Interrupt flag                            | R/W   | All
    Nr10 = 0xff10,  // Sound channel 1 sweep                     | R/W   | All
    Nr11 = 0xff11,  // Sound channel 1 length timer & duty cycle | Mixed | All
    Nr12 = 0xff12,  // Sound channel 1 volume & envelope         | R/W   | All
    Nr13 = 0xff13,  // Sound channel 1 wavelength low            | W     | All
    Nr14 = 0xff14,  // Sound channel 1 wavelength high & control | Mixed | All
    Nr21 = 0xff16,  // Sound channel 2 length timer & duty cycle | Mixed | All
    Nr22 = 0xff17,  // Sound channel 2 volume & envelope         | R/W   | All
    Nr23 = 0xff18,  // Sound channel 2 wavelength low            | W     | All
    Nr24 = 0xff19,  // Sound channel 2 wavelength high & control | Mixed | All
    Nr30 = 0xff1a,  // Sound channel 3 DAC enable                | R/W   | All
    Nr31 = 0xff1b,  // Sound channel 3 length timer              | W     | All
    Nr32 = 0xff1c,  // Sound channel 3 output level              | R/W   | All
    Nr33 = 0xff1d,  // Sound channel 3 wavelength low            | W     | All
    Nr34 = 0xff1e,  // Sound channel 3 wavelength high & control | Mixed | All
    Nr41 = 0xff20,  // Sound channel 4 length timer              | W     | All
    Nr42 = 0xff21,  // Sound channel 4 volume & envelope         | R/W   | All
    Nr43 = 0xff22,  // Sound channel 4 frequency & randomness    | R/W   | All
    Nr44 = 0xff23,  // Sound channel 4 control                   | Mixed | All
    Nr50 = 0xff24,  // Master volume & VIN panning               | R/W   | All
    Nr51 = 0xff25,  // Sound panning                             | R/W   | All
    Nr52 = 0xff26,  // Sound on/off                              | Mixed | All
    Lcdc = 0xff40,  // LCD control                               | R/W   | All
    Stat = 0xff41,  // LCD status                                | Mixed | All
    Scy = 0xff42,   // Viewport Y position                       | R/W   | All
    Scx = 0xff43,   // Viewport X position                       | R/W   | All
    Ly = 0xff44,    // LCD Y coordinate                          | R     | All
    Lyc = 0xff45,   // LY compare                                | R/W   | All
    Dma = 0xff46,   // OAM DMA source address & start            | R/W   | All
    Bgp = 0xff47,   // BG palette data                           | R/W   | DMG
    Obp0 = 0xff48,  // OBJ palette 0 data                        | R/W   | DMG
    Obp1 = 0xff49,  // OBJ palette 1 data                        | R/W   | DMG
    Wy = 0xff4a,    // Window Y position                         | R/W   | All
    Wx = 0xff4b,    // Window X position plus 7                  | R/W   | All
    Key0 = 0xff4c,  // Disable CGB mode; enable compat           | Mixed | CGB
    Key1 = 0xff4d,  // Prepare speed switch                      | Mixed | CGB
    Vbk = 0xff4f,   // VRAM bank                                 | R/W   | CGB
    Bank = 0xff50,  // Write to unmap boot ROM                   | ?     | All
    Hdma1 = 0xff51, // VRAM DMA source high                      | W     | CGB
    Hdma2 = 0xff52, // VRAM DMA source low                       | W     | CGB
    Hdma3 = 0xff53, // VRAM DMA destination high                 | W     | CGB
    Hdma4 = 0xff54, // VRAM DMA destination low                  | W     | CGB
    Hdma5 = 0xff55, // VRAM DMA length/mode/start                | R/W   | CGB
    Rp = 0xff56,    // Infrared communications port              | Mixed | CGB
    Bcps = 0xff68,  // Background color palette specification    | R/W   | CGB
    Bcpd = 0xff69,  // Background color palette data             | R/W   | CGB
    Ocps = 0xff6a,  // OBJ color palette specification           | R/W   | CGB
    Ocpd = 0xff6b,  // OBJ color palette data                    | R/W   | CGB
    Opri = 0xff6c,  // Object priority mode                      | R/W   | CGB
    Svbk = 0xff70,  // WRAM bank                                 | R/W   | CGB
    Pcm12 = 0xff76, // Audio digital outputs 1 & 2               | R     | CGB
    Pcm34 = 0xff77, // Audio digital outputs 3 & 4               | R     | CGB
    Ie = 0xffff,    // Interrupt enable                          | R/W   | All
}

pub type VRam = [[u8; 0x2000]; 2];
pub type PaletteRam = [u8; 64];
pub type Oam = [u8; 0xa0];

struct MemoryData {
    cartrige_rom: [u8; 0x8000], // TODO: MBCs
    vram: VRam,
    cartrige_ram: [u8; 0x2000], // TODO: MBCs
    wram_low: [u8; 0x1000],
    wram_high: [[u8; 0x1000]; 7],
    // echo_ram: mirror of 0xc000~0xddff
    oam: Oam,
    // prohibited_area: 0xfea0~0xfeff
    hram: [u8; 0x100], // HRAM and i/o registers
    bg_palette: PaletteRam,
    obj_palette: PaletteRam,
}

pub struct Memory {
    mem: MemoryData,
    joypad: JoypadState,
    boot_rom_mapped: bool,
    cgb_mode: bool,
    pub cpu_dma_paused: bool,
    pub dma_state: Option<DmaState>,
}

macro_rules! impl_addr_to_ref {
    ($name:ident $( $mut:tt )?) => {
       fn $name(& $( $mut )* self, addr: u16) -> & $( $mut )* u8 {
           match addr {
               0x0000..=0x7fff => & $( $mut )* self.mem.cartrige_rom[addr as usize],
               0x8000..=0x9fff => & $( $mut )* self.mem.vram[if self.cgb_mode {
                   self[MappedReg::Vbk] as usize & 0x1
               } else {
                   0
               }][addr as usize & 0x1fff],
               0xa000..=0xbfff => & $( $mut )* self.mem.cartrige_ram[addr as usize & 0x1fff],
               0xc000..=0xcfff | 0xe000..=0xefff => & $( $mut )* self.mem.wram_low[addr as usize & 0xfff],
               0xd000..=0xdfff | 0xf000..=0xfdff => & $( $mut )* self.mem.wram_high[{
                   let svbk = self[MappedReg::Svbk] as usize & 0x3;
                   if !self.cgb_mode || svbk == 0 { 0 } else { svbk - 1 }
               }][addr as usize & 0xfff],
               0xfe00..=0xfe9f => & $( $mut )* self.mem.oam[addr as usize & 0x9f],
               0xfea0..=0xfeff => panic!("Attempt to access illegal memory area"),
               0xff00..=0xffff => & $( $mut )* self.mem.hram[addr as usize & 0xff],
           }
       }
    };
}

impl Memory {
    pub fn new(rom: impl Into<Vec<u8>>) -> Box<Self> {
        let mut rom = rom.into();
        let mut mem = Box::new(Memory {
            // SAFTEY: All zeros is valid for MemoryData, which is just a bunch of nested arrays of u8
            mem: unsafe { MaybeUninit::<MemoryData>::zeroed().assume_init() },
            joypad: JoypadState::new(),
            boot_rom_mapped: true,
            cgb_mode: true,
            cpu_dma_paused: false,
            dma_state: None,
        });
        rom.resize(mem::size_of_val(&mem.mem.cartrige_rom), 0);
        mem.mem.cartrige_rom.copy_from_slice(&rom[..]);
        mem
    }

    pub fn vram(&self) -> &VRam {
        &self.mem.vram
    }

    pub fn oam(&self) -> &Oam {
        &self.mem.oam
    }

    pub fn bg_palette_ram(&self) -> &PaletteRam {
        &self.mem.bg_palette
    }

    pub fn obj_palette_ram(&self) -> &PaletteRam {
        &self.mem.obj_palette
    }

    pub fn vram_mut(&mut self) -> &mut VRam {
        &mut self.mem.vram
    }

    pub fn oam_mut(&mut self) -> &mut Oam {
        &mut self.mem.oam
    }

    #[cfg(test)]
    pub fn bg_palette_ram_mut(&mut self) -> &mut PaletteRam {
        &mut self.mem.bg_palette
    }

    #[cfg(test)]
    pub fn obj_palette_ram_mut(&mut self) -> &mut PaletteRam {
        &mut self.mem.obj_palette
    }

    impl_addr_to_ref!(addr_to_ref);
    impl_addr_to_ref!(addr_to_ref_mut mut);

    pub fn read_8(&self, addr: u16) -> u8 {
        const BCPD: u16 = MappedReg::Bcpd as _;
        const OCPD: u16 = MappedReg::Ocpd as _;
        const P1: u16 = MappedReg::P1 as _;
        match addr {
            0x0000..=0x00ff | 0x0200..=0x08ff if self.boot_rom_mapped => BOOT_ROM[addr as usize],
            0xfea0..=0xfeff => {
                // CGB-E prohibited area reads, according to pandocs
                let low = addr as u8 & 0x0f;
                low << 4 | low
            }
            BCPD if self.cgb_mode => self.mem.bg_palette[(self[MappedReg::Bcps] & 0x3f) as usize],
            OCPD if self.cgb_mode => self.mem.obj_palette[(self[MappedReg::Ocps] & 0x3f) as usize],
            P1 => {
                let p1 = self[MappedReg::P1];

                let mut bits = 0;
                if (p1 >> 4) & 0x1 == 0 {
                    bits |= self.joypad.direction_bits();
                }
                if (p1 >> 5) & 0x1 == 0 {
                    bits |= self.joypad.action_bits();
                }

                p1 & 0xf0 | !bits & 0x0f
            }
            _ => *self.addr_to_ref(addr),
        }
    }

    fn auto_inc_cps(cps: &mut u8) {
        *cps = (*cps & 0xc0) | cps.wrapping_add(*cps >> 7) & 0x3f;
    }

    pub fn write_8(&mut self, addr: u16, val: u8) {
        const BCPD: u16 = MappedReg::Bcpd as _;
        const OCPD: u16 = MappedReg::Ocpd as _;
        const BANK: u16 = MappedReg::Bank as _;
        const DMA: u16 = MappedReg::Dma as _;
        const HDMA5: u16 = MappedReg::Hdma5 as _;
        match addr {
            0xfea0..=0xfeff => (), // Ignore writes to the prohibited area
            BCPD if self.cgb_mode => {
                self.mem.bg_palette[(self[MappedReg::Bcps] & 0x3f) as usize] = val;
                Self::auto_inc_cps(&mut self[MappedReg::Bcps]);
            }
            OCPD if self.cgb_mode => {
                self.mem.obj_palette[(self[MappedReg::Ocps] & 0x3f) as usize] = val;
                Self::auto_inc_cps(&mut self[MappedReg::Ocps]);
            }
            HDMA5 if self.cgb_mode => {
                if val >> 7 != 0 {
                    todo!("HBlank DMA");
                }
                // TODO: Do some kind of cancel of an ongoing OAM DMA for simplicity
                self.dma_state = Some(DmaState {
                    ty: DmaType::General,
                    len: ((val & 0x7f) as u16).wrapping_add(1) * 16,
                    count: 0,
                    oam_src: 0,
                });
            }
            DMA => {
                // TODO: Do some kind of cancel of an ongoing HDMA for simplicity
                self.dma_state = Some(DmaState {
                    ty: DmaType::Oam,
                    len: 0xa0,
                    count: 0,
                    oam_src: (val as u16) << 8,
                });
            }
            BANK if self.boot_rom_mapped => {
                self.boot_rom_mapped = false;
                self.cgb_mode = self[MappedReg::Key0] != NON_CGB_KEY0_VAL;
            }
            _ => *self.addr_to_ref_mut(addr) = val,
        }
    }

    pub fn read_16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.read_8(addr), self.read_8(addr.wrapping_add(1))])
    }

    pub fn write_16(&mut self, addr: u16, val: u16) {
        let [low, high] = val.to_le_bytes();
        self.write_8(addr, low);
        self.write_8(addr.wrapping_add(1), high);
    }

    pub fn handle_joypad(&mut self, button: Button, state: ButtonState) {
        self.joypad.handle(button, state);
    }

    pub fn cgb_mode(&self) -> bool {
        self.cgb_mode
    }
}

impl Index<MappedReg> for Memory {
    type Output = u8;

    fn index(&self, reg: MappedReg) -> &Self::Output {
        self.addr_to_ref(reg as u16)
    }
}

impl IndexMut<MappedReg> for Memory {
    fn index_mut(&mut self, reg: MappedReg) -> &mut Self::Output {
        self.addr_to_ref_mut(reg as u16)
    }
}
