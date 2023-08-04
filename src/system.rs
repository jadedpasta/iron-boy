use std::{
    mem::{self, MaybeUninit},
    ops::{Index, IndexMut},
};

use partial_borrow::{prelude::*, SplitOff};

use crate::{
    cpu::{Cpu, CpuBus},
    dma::{Dma, DmaBus},
    interrupt::Interrupt,
    joypad::{Button, ButtonState, JoypadState},
    ppu::{Ppu, PpuBus},
    timer::{Timer, TimerBus},
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

macro_rules! impl_addr_to_ref {
    ($name:ident $( $mut:tt )?) => {
       fn $name(& $( $mut )* self, addr: u16, cgb_mode: bool) -> & $( $mut )* u8 {
           match addr {
               0x0000..=0x7fff => & $( $mut )* self.cartrige_rom[addr as usize],
               0x8000..=0x9fff => & $( $mut )* self.vram[if cgb_mode {
                   self[MappedReg::Vbk] as usize & 0x1
               } else {
                   0
               }][addr as usize & 0x1fff],
               0xa000..=0xbfff => & $( $mut )* self.cartrige_ram[addr as usize & 0x1fff],
               0xc000..=0xcfff | 0xe000..=0xefff => & $( $mut )* self.wram_low[addr as usize & 0xfff],
               0xd000..=0xdfff | 0xf000..=0xfdff => & $( $mut )* self.wram_high[{
                   let svbk = self[MappedReg::Svbk] as usize & 0x3;
                   if !cgb_mode || svbk == 0 { 0 } else { svbk - 1 }
               }][addr as usize & 0xfff],
               0xfe00..=0xfe9f => & $( $mut )* self.oam[addr as usize & 0x9f],
               0xfea0..=0xfeff => panic!("Attempt to access illegal memory area"),
               0xff00..=0xffff => & $( $mut )* self.hram[addr as usize & 0xff],
           }
       }
    };
}

impl MemoryData {
    impl_addr_to_ref!(addr_to_ref);
    impl_addr_to_ref!(addr_to_ref_mut mut);
}

impl Index<MappedReg> for MemoryData {
    type Output = u8;

    fn index(&self, reg: MappedReg) -> &Self::Output {
        &self.hram[(reg as u16 & 0x00ff) as usize]
    }
}

impl IndexMut<MappedReg> for MemoryData {
    fn index_mut(&mut self, reg: MappedReg) -> &mut Self::Output {
        &mut self.hram[(reg as u16 & 0x00ff) as usize]
    }
}

#[derive(PartialBorrow)]
pub struct CgbSystem {
    cpu: Cpu,
    pub timer: Timer,
    ppu: Ppu,
    dma: Dma,
    mem: MemoryData,
    joypad: JoypadState,
    boot_rom_mapped: bool,
    cgb_mode: bool,
}

impl CgbSystem {
    pub fn new(rom: impl Into<Vec<u8>>) -> Box<Self> {
        let mut rom = rom.into();
        let mut system = Box::new(CgbSystem {
            cpu: Cpu::default(),
            timer: Timer::new(),
            dma: Dma::new(),
            ppu: Ppu::new(),
            // SAFTEY: All zeros is valid for MemoryData, which is just a bunch of nested arrays of u8
            mem: unsafe { MaybeUninit::<MemoryData>::zeroed().assume_init() },
            joypad: JoypadState::new(),
            boot_rom_mapped: true,
            cgb_mode: true,
        });

        // Registers should be given initial values on startup. Not sure how actual hardware
        // behaves, but this is nice for an emulator.
        let (ppu, bus) = system.split_ppu();
        ppu.update_control_regs(bus);

        rom.resize(mem::size_of_val(&system.mem.cartrige_rom), 0);
        system.mem.cartrige_rom.copy_from_slice(&rom[..]);
        system
    }

    pub fn split_cpu(&mut self) -> (&mut Cpu, &mut impl CpuBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.cpu, bus);
    }

    pub fn split_ppu(&mut self) -> (&mut Ppu, &mut impl PpuBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.ppu, bus);
    }

    pub fn split_dma(&mut self) -> (&mut Dma, &mut impl DmaBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.dma, bus);
    }

    pub fn split_timer(&mut self) -> (&mut Timer, &mut impl TimerBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.timer, bus);
    }

    pub fn lcd_on(&self) -> bool {
        self.mem[MappedReg::Lcdc] & 0x80 != 0
    }

    fn auto_inc_cps(cps: &mut u8) {
        *cps = (*cps & 0xc0) | cps.wrapping_add(*cps >> 7) & 0x3f;
    }

    pub fn handle_joypad(&mut self, button: Button, state: ButtonState) {
        self.joypad.handle(button, state);
    }
}

impl CpuBus for partial!(CgbSystem ! cpu, mut mem timer dma boot_rom_mapped cgb_mode) {
    fn read_8(&self, addr: u16) -> u8 {
        const BCPD: u16 = MappedReg::Bcpd as _;
        const OCPD: u16 = MappedReg::Ocpd as _;
        const P1: u16 = MappedReg::P1 as _;
        const DIV: u16 = MappedReg::Div as _;
        const TIMA: u16 = MappedReg::Tima as _;
        const TMA: u16 = MappedReg::Tma as _;
        const TAC: u16 = MappedReg::Tac as _;
        match addr {
            0x0000..=0x00ff | 0x0200..=0x08ff if *self.boot_rom_mapped => BOOT_ROM[addr as usize],
            0xfea0..=0xfeff => {
                // CGB-E prohibited area reads, according to pandocs
                let low = addr as u8 & 0x0f;
                low << 4 | low
            }
            BCPD if *self.cgb_mode => {
                self.mem.bg_palette[(self.mem[MappedReg::Bcps] & 0x3f) as usize]
            }
            OCPD if *self.cgb_mode => {
                self.mem.obj_palette[(self.mem[MappedReg::Ocps] & 0x3f) as usize]
            }
            P1 => {
                let p1 = self.mem[MappedReg::P1];

                let mut bits = 0;
                if (p1 >> 4) & 0x1 == 0 {
                    bits |= self.joypad.direction_bits();
                }
                if (p1 >> 5) & 0x1 == 0 {
                    bits |= self.joypad.action_bits();
                }

                p1 & 0xf0 | !bits & 0x0f
            }
            DIV => self.timer.div(),
            TIMA => self.timer.tima(),
            TMA => self.timer.tma(),
            TAC => self.timer.tac(),
            _ => *self.mem.addr_to_ref(addr, *self.cgb_mode),
        }
    }

    fn write_8(&mut self, addr: u16, val: u8) {
        const BCPD: u16 = MappedReg::Bcpd as _;
        const OCPD: u16 = MappedReg::Ocpd as _;
        const BANK: u16 = MappedReg::Bank as _;
        const DMA: u16 = MappedReg::Dma as _;
        const HDMA5: u16 = MappedReg::Hdma5 as _;
        const DIV: u16 = MappedReg::Div as _;
        const TIMA: u16 = MappedReg::Tima as _;
        const TMA: u16 = MappedReg::Tma as _;
        const TAC: u16 = MappedReg::Tac as _;
        match addr {
            0x0000..=0x7fff => (), // Ignore writes to cartridge ROM (TODO: MBCs)
            0xfea0..=0xfeff => (), // Ignore writes to the prohibited area
            BCPD if *self.cgb_mode => {
                let bcps = self.mem[MappedReg::Bcps];
                self.mem.bg_palette[(bcps & 0x3f) as usize] = val;
                CgbSystem::auto_inc_cps(&mut self.mem[MappedReg::Bcps]);
            }
            OCPD if *self.cgb_mode => {
                let ocps = self.mem[MappedReg::Ocps];
                self.mem.obj_palette[(ocps & 0x3f) as usize] = val;
                CgbSystem::auto_inc_cps(&mut self.mem[MappedReg::Ocps]);
            }
            HDMA5 if *self.cgb_mode => {
                let len = ((val & 0x7f) as u16).wrapping_add(1) * 16;
                if val >> 7 != 0 {
                    todo!("HBlank DMA");
                } else {
                    self.dma.start_general(len);
                }
            }
            DMA => {
                self.dma.start_oam((val as u16) << 8);
            }
            BANK if *self.boot_rom_mapped => {
                *self.boot_rom_mapped = false;
                *self.cgb_mode = self.mem[MappedReg::Key0] != NON_CGB_KEY0_VAL;
            }
            DIV => self.timer.reset_div(),
            TIMA => self.timer.set_tima(val),
            TMA => self.timer.set_tma(val),
            TAC => self.timer.set_tac(val),
            _ => *self.mem.addr_to_ref_mut(addr, *self.cgb_mode) = val,
        }
    }

    fn cpu_dma_paused(&self) -> bool {
        self.dma.cpu_paused()
    }

    fn pop_interrupt(&mut self) -> Option<u8> {
        let pending = self.mem[MappedReg::Ie] & self.mem[MappedReg::If];
        let bit = pending.trailing_zeros() as u8;
        if bit > 7 {
            // No interrupts are pending.
            return None;
        }
        // Toggle off the flag bit to mark the interrupt as handled.
        self.mem[MappedReg::If] ^= 1 << bit;
        Some(bit)
    }
}

impl PpuBus for partial!(CgbSystem ! ppu, mut mem) {
    fn lcdc(&self) -> u8 {
        self.mem[MappedReg::Lcdc]
    }

    fn scx(&self) -> u8 {
        self.mem[MappedReg::Scx]
    }

    fn scy(&self) -> u8 {
        self.mem[MappedReg::Scy]
    }

    fn bgp(&self) -> u8 {
        self.mem[MappedReg::Bgp]
    }

    fn obp0(&self) -> u8 {
        self.mem[MappedReg::Obp0]
    }

    fn obp1(&self) -> u8 {
        self.mem[MappedReg::Obp1]
    }

    fn set_ly(&mut self, ly: u8) {
        self.mem[MappedReg::Ly] = ly;
    }

    fn set_stat(&mut self, stat: u8) {
        self.mem[MappedReg::Stat] = stat;
    }

    fn trigger_vblank_interrupt(&mut self) {
        Interrupt::VBlank.request(&mut self.mem[MappedReg::If]);
    }

    fn vram(&self) -> &VRam {
        &self.mem.vram
    }

    fn bg_palette_ram(&self) -> &PaletteRam {
        &self.mem.bg_palette
    }

    fn obj_palette_ram(&self) -> &PaletteRam {
        &self.mem.obj_palette
    }

    fn oam(&self) -> &Oam {
        &self.mem.oam
    }

    fn cgb_mode(&self) -> bool {
        *self.cgb_mode
    }
}

impl DmaBus for partial!(CgbSystem ! dma, mut mem) {
    fn general_src_addr(&self) -> u16 {
        let hdma1 = self.mem[MappedReg::Hdma1] as u16;
        let hdma2 = self.mem[MappedReg::Hdma2] as u16;
        ((hdma1 << 8) | hdma2) & 0xfff0
    }

    fn general_dst_addr(&self) -> u16 {
        let hdma3 = self.mem[MappedReg::Hdma3] as u16;
        let hdma4 = self.mem[MappedReg::Hdma4] as u16;
        ((hdma3 << 8) | hdma4) & 0x1ff0
    }

    fn vbk(&self) -> usize {
        self.mem[MappedReg::Vbk] as usize & 0x1
    }

    fn vram_mut(&mut self) -> &mut VRam {
        &mut self.mem.vram
    }

    fn oam_mut(&mut self) -> &mut Oam {
        &mut self.mem.oam
    }

    fn read_8(&self, addr: u16) -> u8 {
        *self.mem.addr_to_ref(addr, *self.cgb_mode)
    }
}

impl TimerBus for partial!(CgbSystem ! timer, mut mem) {
    fn request_timer_interrupt(&mut self) {
        Interrupt::Timer.request(&mut self.mem[MappedReg::If]);
    }
}
