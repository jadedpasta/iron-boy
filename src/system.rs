use partial_borrow::{prelude::*, SplitOff};

use crate::{
    cpu::{Cpu, CpuBus},
    dma::{Dma, DmaBus},
    interrupt::{Interrupt, InterruptState},
    joypad::{Button, ButtonState, Joypad},
    memory::{MemoryData, OamBytes, Palettes, VRamBytes},
    ppu::{Ppu, PpuBus},
    timer::{Timer, TimerBus},
};

const BOOT_ROM: &'static [u8] = include_bytes!("../sameboy_boot.bin");
const NON_CGB_KEY0_VAL: u8 = 0x04;

mod reg {
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
}

#[derive(PartialBorrow)]
pub struct CgbSystem {
    cpu: Cpu,
    pub timer: Timer,
    ppu: Ppu,
    dma: Dma,
    mem: MemoryData,
    joypad: Joypad,
    interrupt: InterruptState,
    boot_rom_mapped: bool,
    cgb_mode: bool,
    key0: u8, // TODO: This can probably be combined with cgb_mode
}

impl CgbSystem {
    pub fn new(rom: impl Into<Vec<u8>>) -> Box<Self> {
        Box::new(CgbSystem {
            cpu: Cpu::default(),
            timer: Timer::new(),
            dma: Dma::new(),
            ppu: Ppu::new(),
            mem: MemoryData::new(rom),
            joypad: Joypad::new(),
            interrupt: InterruptState::new(),
            boot_rom_mapped: true,
            cgb_mode: true,
            key0: 0,
        })
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
        self.ppu.lcdc & 0x80 != 0
    }

    pub fn handle_joypad(&mut self, button: Button, state: ButtonState) {
        self.joypad.handle(button, state);
    }
}

impl CpuBus for partial!(CgbSystem ! cpu, mut mem ppu timer dma joypad interrupt boot_rom_mapped cgb_mode key0) {
    fn read_8(&self, addr: u16) -> u8 {
        match (addr >> 8) as u8 {
            0x00..=0x00 | 0x02..=0x08 if *self.boot_rom_mapped => BOOT_ROM[addr as usize],
            0x00..=0x7f => self.mem.cartrige_rom[addr as usize],
            0x80..=0x9f => self.mem.vram.read(addr, *self.cgb_mode),
            0xa0..=0xbf => self.mem.cartrige_ram[addr as usize & 0x1fff],
            0xc0..=0xcf | 0xe0..=0xef => self.mem.wram.read_low(addr),
            0xd0..=0xdf | 0xf0..=0xfd => self.mem.wram.read_high(addr, *self.cgb_mode),
            0xfe => match addr as u8 {
                low @ 0x00..=0x9f => self.mem.oam[low as usize],
                low @ 0xa0..=0xff => {
                    // CGB-E prohibited area reads, according to pandocs
                    let low = low & 0x0f;
                    low << 4 | low
                }
            },
            0xff => match addr as u8 {
                low @ 0x80..=0xfe => self.mem.hram[low as usize - 0x80],
                reg::BCPD if *self.cgb_mode => self.mem.bg_palette.read_data(),
                reg::OCPD if *self.cgb_mode => self.mem.obj_palette.read_data(),
                reg::BCPS if *self.cgb_mode => self.mem.bg_palette.select,
                reg::OCPS if *self.cgb_mode => self.mem.obj_palette.select,
                reg::HDMA5 if *self.cgb_mode => self.dma.hdma5(),
                reg::HDMA1 => self.dma.hdma1,
                reg::HDMA2 => self.dma.hdma2,
                reg::HDMA3 => self.dma.hdma3,
                reg::HDMA4 => self.dma.hdma4,
                reg::P1 => self.joypad.p1(),
                reg::DIV => self.timer.div(),
                reg::TIMA => self.timer.tima(),
                reg::TMA => self.timer.tma(),
                reg::TAC => self.timer.tac(),
                reg::SVBK => self.mem.wram.svbk,
                reg::VBK => self.mem.vram.vbk,
                reg::IF => self.interrupt.flags,
                reg::IE => self.interrupt.enable,
                reg::DMA => self.dma.dma(),
                reg::BGP => self.ppu.bgp,
                reg::LCDC => self.ppu.lcdc,
                reg::LY => self.ppu.ly,
                reg::OBP0 => self.ppu.obp0,
                reg::OBP1 => self.ppu.obp1,
                reg::SCX => self.ppu.scx,
                reg::SCY => self.ppu.scy,
                reg::STAT => self.ppu.stat(),
                _ => 0, // unimplemented
            },
        }
    }

    fn write_8(&mut self, addr: u16, val: u8) {
        match (addr >> 8) as u8 {
            0x00..=0x7f => (), // Ignore writes to cartridge ROM (TODO: MBCs)
            0x80..=0x9f => self.mem.vram.write(addr, val, *self.cgb_mode),
            0xa0..=0xbf => self.mem.cartrige_ram[addr as usize & 0x1fff] = val,
            0xc0..=0xcf | 0xe0..=0xef => self.mem.wram.write_low(addr, val),
            0xd0..=0xdf | 0xf0..=0xfd => self.mem.wram.write_high(addr, val, *self.cgb_mode),
            0xfe => match addr as u8 {
                low @ 0x00..=0x9f => self.mem.oam[low as usize] = val,
                0xa0..=0xff => (),
            },
            0xff => match addr as u8 {
                low @ 0x80..=0xfe => self.mem.hram[low as usize - 0x80] = val,
                reg::BCPD if *self.cgb_mode => self.mem.bg_palette.write_data(val),
                reg::OCPD if *self.cgb_mode => self.mem.obj_palette.write_data(val),
                reg::BCPS if *self.cgb_mode => self.mem.bg_palette.select = val,
                reg::OCPS if *self.cgb_mode => self.mem.obj_palette.select = val,
                reg::HDMA5 if *self.cgb_mode => self.dma.set_hdma5(val),
                reg::DMA => self.dma.set_dma(val),
                reg::BANK if *self.boot_rom_mapped => {
                    *self.boot_rom_mapped = false;
                    *self.cgb_mode = *self.key0 != NON_CGB_KEY0_VAL;
                }
                reg::KEY0 => *self.key0 = val,
                reg::HDMA1 => self.dma.hdma1 = val,
                reg::HDMA2 => self.dma.hdma2 = val,
                reg::HDMA3 => self.dma.hdma3 = val,
                reg::HDMA4 => self.dma.hdma4 = val,
                reg::DIV => self.timer.reset_div(),
                reg::TIMA => self.timer.set_tima(val),
                reg::TMA => self.timer.set_tma(val),
                reg::TAC => self.timer.set_tac(val),
                reg::SVBK => self.mem.wram.svbk = val,
                reg::VBK => self.mem.vram.vbk = val,
                reg::P1 => self.joypad.set_p1(val),
                reg::IF => self.interrupt.flags = val,
                reg::IE => self.interrupt.enable = val,
                reg::BGP => self.ppu.bgp = val,
                reg::LCDC => self.ppu.lcdc = val,
                reg::LY => self.ppu.ly = val,
                reg::OBP0 => self.ppu.obp0 = val,
                reg::OBP1 => self.ppu.obp1 = val,
                reg::SCX => self.ppu.scx = val,
                reg::SCY => self.ppu.scy = val,
                reg::STAT => self.ppu.set_stat(val),
                _ => (), // unimplemented
            },
        }
    }

    fn cpu_dma_paused(&self) -> bool {
        self.dma.cpu_paused()
    }

    fn pop_interrupt(&mut self) -> Option<u8> {
        self.interrupt.pop()
    }
}

impl PpuBus for partial!(CgbSystem ! ppu, mut mem interrupt) {
    fn trigger_vblank_interrupt(&mut self) {
        self.interrupt.request(Interrupt::VBlank);
    }

    fn vram(&self) -> &VRamBytes {
        &self.mem.vram.bytes()
    }

    fn bg_palette_ram(&self) -> &Palettes {
        &self.mem.bg_palette.palettes()
    }

    fn obj_palette_ram(&self) -> &Palettes {
        &self.mem.obj_palette.palettes()
    }

    fn oam(&self) -> &OamBytes {
        &self.mem.oam
    }

    fn cgb_mode(&self) -> bool {
        *self.cgb_mode
    }
}

impl DmaBus for partial!(CgbSystem ! dma, mut mem) {
    fn write_vram(&mut self, addr: u16, val: u8) {
        self.mem.vram.write(addr, val, *self.cgb_mode);
    }

    fn oam_mut(&mut self) -> &mut OamBytes {
        &mut self.mem.oam
    }

    fn read_8(&self, addr: u16) -> u8 {
        match (addr >> 8) as u8 {
            0x00..=0x00 | 0x02..=0x08 if *self.boot_rom_mapped => BOOT_ROM[addr as usize],
            0x00..=0x7f => self.mem.cartrige_rom[addr as usize],
            0x80..=0x9f => self.mem.vram.read(addr, *self.cgb_mode),
            0xa0..=0xbf => self.mem.cartrige_ram[addr as usize & 0x1fff],
            0xc0..=0xcf | 0xe0..=0xef => self.mem.wram.read_low(addr),
            0xd0..=0xdf | 0xf0..=0xff => self.mem.wram.read_high(addr, *self.cgb_mode),
        }
    }
}

impl TimerBus for partial!(CgbSystem ! timer, mut mem interrupt) {
    fn request_timer_interrupt(&mut self) {
        self.interrupt.request(Interrupt::Timer);
    }
}
