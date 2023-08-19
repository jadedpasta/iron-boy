// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
use bilge::prelude::*;

use crate::{
    memory::{OamBytes, Palettes, VRamBytes},
    Cgb, FrameBuffer,
};

#[bitsize(2)]
#[derive(FromBits, Debug, Default, Clone, Copy)]
enum Mode {
    HBlank,
    VBlank,
    #[default]
    OamSearch,
    Transfer,
}

impl Mode {
    const fn cycles(&self) -> usize {
        match self {
            Self::OamSearch => 21,
            Self::Transfer => 43,
            Self::HBlank => 50,
            Self::VBlank => 114,
        }
    }
}

#[bitsize(4)]
#[derive(FromBits, DebugBits, DefaultBits, Clone, Copy)]
struct StatInterruptSources {
    hblank: bool,
    vblank: bool,
    oam: bool,
    lyc_equal: bool,
}

#[bitsize(8)]
#[derive(FromBits, DebugBits, DefaultBits, Clone, Copy)]
struct Stat {
    mode: Mode,
    lyc_equal: bool,
    int_sources: StatInterruptSources,
    __: u1,
}

#[bitsize(8)]
#[derive(FromBits, DebugBits, Clone, Copy)]
struct Lcdc {
    bg_window_enable_priority: bool,
    obj_enabled: bool,
    tall_obj_enabled: bool,
    bg_map_bit: u1,
    tile_data_bit: u1,
    window_enabled: bool,
    window_map_bit: u1,
    lcd_enabled: bool,
}

#[bitsize(8)]
#[derive(DebugBits, Clone, Copy)]
#[repr(transparent)]
pub struct ObjAttrs {
    palette: u3,
    bank: u1,
    palette_dmg: u1,
    x_flipped: bool,
    y_flipped: bool,
    bg_over_obj: bool,
}

#[derive(Debug)]
#[repr(C, packed)]
struct Obj {
    y: u8,
    x: u8,
    tile: u8,
    attrs: ObjAttrs,
}

type Objs = [Obj; 40];

pub trait PpuBus {
    fn request_vblank_interrupt(&mut self);
    fn request_stat_interrupt(&mut self);

    fn vram(&self) -> &VRamBytes;
    fn bg_palette_ram(&self) -> &Palettes;
    fn obj_palette_ram(&self) -> &Palettes;
    fn oam(&self) -> &OamBytes;

    fn cgb_mode(&self) -> bool;
}

// Use a separate extension trait so that Obj can be private
trait ObjView: PpuBus {
    fn objs(&self) -> &Objs {
        let oam = self.oam();
        unsafe { &*(oam as *const _ as *const _) }
    }
}
impl<T: PpuBus> ObjView for T {}

#[derive(Debug)]
pub struct Ppu {
    mode_cycles_remaining: usize,
    pub bgp: u8,
    lcdc: Lcdc,
    ly: u8,
    pub lyc: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub scx: u8,
    pub scy: u8,
    stat: Stat,
    interrupt_line: bool,
}

struct ObjPixel {
    color: u8,
    palette: u8,
    bg_over_obj: bool,
}

struct BgPixel {
    color: u8,
    palette: u8,
    bg_over_obj: bool,
}

impl Ppu {
    pub fn new() -> Self {
        let stat = Stat::default();
        Self {
            mode_cycles_remaining: stat.mode().cycles(),
            bgp: 0,
            lcdc: Lcdc::from(0),
            ly: 0,
            lyc: 0,
            obp0: 0,
            obp1: 0,
            scx: 0,
            scy: 0,
            stat,
            interrupt_line: false,
        }
    }

    fn fetch_bg_pixel(&self, lx: u8, pixel_y: u8, tile_y: u8, bus: &impl PpuBus) -> BgPixel {
        let vram = bus.vram();

        let pixel_x = lx.wrapping_add(self.scx);
        // Compute the tilemap address
        let map_area_bit = self.lcdc.bg_map_bit().value() as usize;
        let tile_x = pixel_x / 8;
        let vram_addr = 0x1800 | (map_area_bit << 10) | ((tile_y as usize) << 5) | tile_x as usize;
        // Grab the tile ID and attributes from the tile map
        let tile_id = vram[0][vram_addr];
        let attributes = vram[1][vram_addr];

        // Grab the pixel data corresponding to that tile ID
        let y_offset = pixel_y & 0x7;
        let addr_mode_bit = !(self.lcdc.tile_data_bit().value() | (tile_id >> 7)) & 0x1;
        let vram_addr = ((addr_mode_bit as usize) << 12)
            | ((tile_id as usize) << 4)
            | ((y_offset as usize) << 1);
        let bank = bus.cgb_mode() as u8 & (attributes >> 3) & 0x1;
        let vram_bank = &vram[bank as usize];
        let color_low = vram_bank[vram_addr];
        let color_high = vram_bank[vram_addr + 1];

        // Convert the data and render it to the screen
        let color_bit = 7 - (pixel_x & 0x7);
        let color_low = (color_low >> color_bit) & 0x1;
        let color_high = (color_high >> color_bit) & 0x1;
        let color = (color_high << 1) | color_low;

        let palette = if bus.cgb_mode() { attributes & 0x7 } else { 0 };
        BgPixel { color, palette, bg_over_obj: attributes & 0x80 != 0 }
    }

    fn fetch_obj_pixel(
        &self,
        lx: u8,
        target_y: u8,
        selected_objs: &[usize],
        bus: &impl PpuBus,
    ) -> Option<ObjPixel> {
        if !self.lcdc.obj_enabled() {
            return None;
        }

        let vram = bus.vram();
        let target_x = lx + 8;

        for obj in selected_objs
            .iter()
            .map(|i| &bus.objs()[*i])
            .filter(|obj| obj.x <= target_x && target_x < obj.x + 8)
        {
            let x_flip = obj.attrs.x_flipped();
            let y_flip = obj.attrs.y_flipped();
            let (tile_id, tile_y) = if self.lcdc.tall_obj_enabled() {
                // 8x16 mode

                // The bottom tile is 8px below the start of the sprite
                let bottom_tile_y = obj.y + 8;

                // We are rendering the bottom of the sprite if the target Y is in the bottom tile
                let bottom_tile = target_y >= bottom_tile_y;

                // The tile ID should be offset by 1 for the bottom tile, unless the OBJ is also
                // y-flipped. LSB of the tile ID is ignored.
                let tile_id = obj.tile & 0xfe | ((bottom_tile ^ y_flip) as u8);

                let tile_y = if bottom_tile { bottom_tile_y } else { obj.y };

                (tile_id, tile_y)
            } else {
                // 8x8 mode
                (obj.tile, obj.y)
            };

            let mut y_offset = target_y - tile_y;
            if y_flip {
                y_offset = 7 - y_offset;
            }

            let vram_addr = ((tile_id as usize) << 4) | ((y_offset as usize) << 1);
            let bank = if bus.cgb_mode() { obj.attrs.bank().value() as usize } else { 0 };
            let vram_bank = &vram[bank];
            let color_low = vram_bank[vram_addr];
            let color_high = vram_bank[vram_addr + 1];

            let mut color_bit = target_x - obj.x;
            if !x_flip {
                color_bit = 7 - color_bit;
            }
            let color_low = (color_low >> color_bit) & 0x1;
            let color_high = (color_high >> color_bit) & 0x1;
            let color = (color_high << 1) | color_low;

            if color == 0 {
                // color 0 is transparent for OBJs. There could be another OBJ overlapping; try the
                // next one
                continue;
            }

            return Some(ObjPixel {
                color,
                palette: if bus.cgb_mode() {
                    obj.attrs.palette().value()
                } else {
                    obj.attrs.palette_dmg().value()
                },
                bg_over_obj: obj.attrs.bg_over_obj(),
            });
        }
        None
    }

    fn mix_pixels(&self, bg_pixel: BgPixel, obj_pixel: Option<ObjPixel>, bus: &impl PpuBus) -> u16 {
        let bg_palettes = bus.bg_palette_ram();
        let obj_palettes = bus.obj_palette_ram();

        let bg_enable_pri = self.lcdc.bg_window_enable_priority();
        if let Some(obj_pixel) = obj_pixel {
            let obj_priority = bg_pixel.color == 0
                || if bus.cgb_mode() {
                    !bg_enable_pri || !bg_pixel.bg_over_obj && !obj_pixel.bg_over_obj
                } else {
                    !obj_pixel.bg_over_obj
                };
            if obj_priority {
                let (color, palette) = if bus.cgb_mode() {
                    (obj_pixel.color, obj_pixel.palette)
                } else {
                    let obp = if obj_pixel.palette == 0 { self.obp0 } else { self.obp1 };
                    ((obp >> (obj_pixel.color * 2)) & 0x3, obj_pixel.palette)
                };

                let palette = obj_palettes[palette as usize];
                return u16::from_le_bytes(palette[color as usize]);
            }
        }

        if !bus.cgb_mode() && !bg_enable_pri {
            // BG disabled; display as white
            return 0x7fff;
        }

        let color =
            if bus.cgb_mode() { bg_pixel.color } else { (self.bgp >> (bg_pixel.color * 2)) & 0x3 };

        let palette = bg_palettes[bg_pixel.palette as usize];
        u16::from_le_bytes(palette[color as usize])
    }

    fn draw_scanline(&self, frame_buff: &mut FrameBuffer, bus: &impl PpuBus) {
        // OAM Search
        let objs = bus.objs();
        let height = match self.lcdc.tall_obj_enabled() {
            true => 16,
            false => 8,
        };
        let obj_target_y = self.ly + 16;
        let mut selected_objs: Vec<usize> = objs
            .iter()
            .enumerate()
            .filter(|(_, obj)| obj.y <= obj_target_y && obj_target_y < obj.y + height)
            .map(|(i, _)| i)
            .take(10)
            .collect();

        if !bus.cgb_mode() {
            // In compatibility mode, objs with smaller x-coordinate have higher priority. A stable
            // sort is required.
            selected_objs.sort_by_key(|i| objs[*i].x);
        }

        let pixel_y = self.ly.wrapping_add(self.scy);
        let tile_y = pixel_y / 8;
        for lx in 0..Cgb::SCREEN_WIDTH as u8 {
            let obj_pixel = self.fetch_obj_pixel(lx, obj_target_y, &selected_objs, bus);

            let bg_pixel = self.fetch_bg_pixel(lx, pixel_y, tile_y, bus);

            let color = self.mix_pixels(bg_pixel, obj_pixel, bus);

            let mask_rescale = |c| ((c & 0x1f) * 0xff / 0x1f) as u8;
            let red = mask_rescale(color);
            let green = mask_rescale(color >> 5);
            let blue = mask_rescale(color >> 10);
            frame_buff[self.ly as usize][lx as usize] = [red, green, blue, 0xff];
        }
    }

    fn switch_mode(&mut self, mode: Mode) {
        self.mode_cycles_remaining = mode.cycles();
        self.stat.set_mode(mode)
    }

    pub fn stat(&self) -> u8 {
        if self.lcd_enabled() {
            self.stat.into()
        } else {
            0
        }
    }

    pub fn set_stat(&mut self, stat: u8) {
        let stat = Stat::from(stat);
        self.stat.set_int_sources(stat.int_sources())
    }

    pub fn ly(&self) -> u8 {
        self.ly
    }

    pub fn lcdc(&self) -> u8 {
        self.lcdc.into()
    }

    pub fn lcd_enabled(&self) -> bool {
        self.lcdc.lcd_enabled()
    }

    pub fn set_lcdc(&mut self, lcdc: u8) {
        self.lcdc = Lcdc::from(lcdc);

        if !self.lcdc.lcd_enabled() {
            self.ly = 0;
            self.switch_mode(Mode::OamSearch);
            self.interrupt_line = false;
        }
    }

    pub fn execute(&mut self, frame_buff: &mut FrameBuffer, bus: &mut impl PpuBus) {
        if !self.lcd_enabled() {
            return;
        }

        if self.mode_cycles_remaining > 1 {
            // There are still cycles left for the current mode. Wait to do anything until the last
            // cycle.
            self.mode_cycles_remaining -= 1;
            return;
        }
        self.mode_cycles_remaining = 0;

        match self.stat.mode() {
            Mode::OamSearch => self.switch_mode(Mode::Transfer),
            Mode::Transfer => {
                self.draw_scanline(frame_buff, bus);
                self.switch_mode(Mode::HBlank);
            }
            Mode::HBlank => {
                self.ly += 1;
                self.switch_mode(if self.ly == Cgb::SCREEN_HEIGHT as u8 {
                    bus.request_vblank_interrupt();
                    Mode::VBlank
                } else {
                    Mode::OamSearch
                });
            }
            Mode::VBlank => {
                self.ly += 1;
                if self.ly == Cgb::FRAME_LINES as u8 {
                    self.ly = 0;
                    self.switch_mode(Mode::OamSearch);
                } else {
                    self.mode_cycles_remaining = Mode::VBlank.cycles();
                }
            }
        }

        let lyc_equal = self.ly == self.lyc;
        self.stat.set_lyc_equal(lyc_equal);

        let int_sources = self.stat.int_sources();

        let interrupt_line = (lyc_equal && int_sources.lyc_equal()) || {
            match self.stat.mode() {
                Mode::Transfer => false,
                Mode::HBlank => int_sources.hblank(),
                Mode::VBlank => int_sources.vblank(),
                Mode::OamSearch => int_sources.oam(),
            }
        };

        if interrupt_line && !self.interrupt_line {
            // "STAT blocking": only request interrupts on the rising edge
            bus.request_stat_interrupt();
        }
        self.interrupt_line = interrupt_line;
    }
}

#[cfg(test)]
mod tests {
    use std::{iter::repeat, mem::MaybeUninit};

    use crate::memory::VRamBytes;

    use super::*;

    struct Bus {
        vram: VRamBytes,
        bg_palette_ram: Palettes,
        obj_palette_ram: Palettes,
        oam: OamBytes,
        cgb_mode: bool,
    }

    impl Bus {
        fn new() -> Box<Self> {
            Box::new(Self {
                vram: unsafe { MaybeUninit::zeroed().assume_init() },
                bg_palette_ram: unsafe { MaybeUninit::zeroed().assume_init() },
                obj_palette_ram: unsafe { MaybeUninit::zeroed().assume_init() },
                oam: unsafe { MaybeUninit::zeroed().assume_init() },
                cgb_mode: true,
            })
        }
    }

    impl PpuBus for Bus {
        fn request_vblank_interrupt(&mut self) {}
        fn request_stat_interrupt(&mut self) {}

        fn vram(&self) -> &VRamBytes {
            &self.vram
        }

        fn bg_palette_ram(&self) -> &Palettes {
            &self.bg_palette_ram
        }

        fn obj_palette_ram(&self) -> &Palettes {
            &self.obj_palette_ram
        }

        fn oam(&self) -> &OamBytes {
            &self.oam
        }

        fn cgb_mode(&self) -> bool {
            self.cgb_mode
        }
    }

    struct Context {
        ppu: Ppu,
        bus: Box<Bus>,
        frame_buff: FrameBuffer,
    }

    impl Context {
        fn new(vram_init: impl FnOnce(&mut VRamBytes)) -> Self {
            let mut bus = Bus::new();
            vram_init(&mut bus.vram);
            let palette: Vec<[u8; 2]> =
                [0xffff, 0x1f << 10, 0x1f << 5, 0x1f].into_iter().map(u16::to_le_bytes).collect();
            bus.bg_palette_ram[0].copy_from_slice(&palette);
            let mut ppu = Ppu::new();
            ppu.lcdc.set_lcd_enabled(true);
            ppu.lcdc.set_tile_data_bit(true.into());
            Self { ppu, bus, frame_buff: unsafe { MaybeUninit::zeroed().assume_init() } }
        }

        fn draw_frame(&mut self) {
            let mode = self.ppu.stat.mode();
            assert!(mode as u8 == Mode::OamSearch as u8, "Started frame in {mode:?}");
            for _ in 0..Cgb::DOTS_PER_FRAME / 4 {
                self.ppu.execute(&mut self.frame_buff, &mut *self.bus);
            }
        }

        fn assert_frame(&self, mut pixel_func: impl FnMut(u8, u8) -> [u8; 3]) {
            for (y, (x, pixel)) in self
                .frame_buff
                .iter()
                .enumerate()
                .flat_map(|(y, row)| repeat(y).zip(row.iter().enumerate()))
            {
                let [r, g, b] = pixel_func(x as u8, y as u8);
                assert_eq!(pixel, &[r, g, b, 0xff], "pos: ({x}, {y})");
            }
        }
    }

    fn checkerboard_vram_init(vram: &mut VRamBytes) {
        vram[0][0..16].copy_from_slice(&[0xff; 16]);
        vram[0][16..32].copy_from_slice(&[0x00; 16]);
        for (y, x) in (0..32).flat_map(|y| repeat(y).zip(0..32)) {
            let addr = 0x1800 + 32 * y + x;
            vram[0][addr] = if x & 0x1 == y & 0x1 { 0x00 } else { 0x01 };
            vram[1][addr] = 0x00;
        }
    }

    #[test]
    fn scroll_x() {
        let mut ctx = Context::new(checkerboard_vram_init);
        for scx in 0..=255 {
            ctx.ppu.scx = scx;
            ctx.draw_frame();
            ctx.assert_frame(|x, y| {
                let tile_x = x.wrapping_add(scx) / 8;
                let tile_y = y / 8;
                if tile_x & 0x1 == tile_y & 0x1 {
                    [0xff, 0x00, 0x00]
                } else {
                    [0xff, 0xff, 0xff]
                }
            });
        }
    }

    #[test]
    fn scroll_y() {
        let mut ctx = Context::new(checkerboard_vram_init);
        for scy in 0..=255 {
            ctx.ppu.scy = scy;
            ctx.draw_frame();
            ctx.assert_frame(|x, y| {
                let tile_x = x / 8;
                let tile_y = y.wrapping_add(scy) / 8;
                if tile_x & 0x1 == tile_y & 0x1 {
                    [0xff, 0x00, 0x00]
                } else {
                    [0xff, 0xff, 0xff]
                }
            });
        }
    }
}
