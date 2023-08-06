use partial_borrow::{prelude::*, SplitOff};

use crate::{
    cpu::{Cpu, CpuBus},
    dma::{Dma, DmaBus},
    interrupt::{Interrupt, InterruptState},
    joypad::{Button, ButtonState, Joypad},
    memory::{MemoryData, OamBytes, Palettes, VRamBytes},
    ppu::{Ppu, PpuBus},
    reg,
    timer::{Timer, TimerBus},
};

const BOOT_ROM: &'static [u8] = include_bytes!("../sameboy_boot.bin");
const NON_CGB_KEY0_VAL: u8 = 0x04;

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
