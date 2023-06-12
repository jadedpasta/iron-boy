use std::mem;

use crate::{
    interrupt::Interrupt,
    memory::{MappedReg, Memory, Oam, PaletteRam},
    Cgb, FrameBuffer,
};

#[derive(Debug, Default, Clone, Copy)]
enum ModeState {
    HBlank = 0,
    VBlank,
    #[default]
    OamSearch,
    Transfer,
}

impl ModeState {
    const fn cycles(&self) -> usize {
        match self {
            Self::OamSearch => 21,
            Self::Transfer => 43,
            Self::HBlank => 50,
            Self::VBlank => 114,
        }
    }
}

#[derive(Debug)]
pub struct Ppu {
    mode_state: ModeState,
    ly: u8,
    mode_cycles_remaining: usize,
}

type Color = [u8; 2];
type Palette = [Color; 4];
type Palettes = [Palette; 8];
type Obj = [u8; 4];
type Objs = [Obj; 40];

fn ram_to_palettes(ram: &PaletteRam) -> &Palettes {
    unsafe { mem::transmute(ram) }
}

fn ram_to_objs(ram: &Oam) -> &Objs {
    unsafe { mem::transmute(ram) }
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
    pub fn new(mem: &mut Memory) -> Self {
        let mode_state = ModeState::default();
        let mut result = Self { mode_cycles_remaining: mode_state.cycles(), mode_state, ly: 0 };
        // Registers should be given initial values on startup. Not sure how actual hardware
        // behaves, but this is nice for an emulator.
        result.update_control_regs(mem);
        result
    }

    fn fetch_bg_pixel(&self, lx: u8, pixel_y: u8, tile_y: u8, mem: &Memory) -> BgPixel {
        let lcdc = mem[MappedReg::Lcdc];
        let scx = mem[MappedReg::Scx];
        let vram = mem.vram();

        let pixel_x = lx.wrapping_add(scx);
        // Compute the tilemap address
        let map_area_bit = ((lcdc >> 3) & 0x1) as usize;
        let tile_x = pixel_x / 8;
        let vram_addr = 0x1800 | (map_area_bit << 10) | ((tile_y as usize) << 5) | tile_x as usize;
        // Grab the tile ID and attributes from the tile map
        let tile_id = vram[0][vram_addr];
        let attributes = vram[1][vram_addr];

        // Grab the pixel data corresponding to that tile ID
        let y_offset = pixel_y & 0x7;
        let addr_mode_bit = !((lcdc >> 4) | (tile_id >> 7)) & 0x1;
        let vram_addr = ((addr_mode_bit as usize) << 12)
            | ((tile_id as usize) << 4)
            | ((y_offset as usize) << 1);
        let bank = mem.cgb_mode() as u8 & (attributes >> 3) & 0x1;
        let vram_bank = &vram[bank as usize];
        let color_low = vram_bank[vram_addr];
        let color_high = vram_bank[vram_addr + 1];

        // Convert the data and render it to the screen
        let color_bit = 7 - (pixel_x & 0x7);
        let color_low = (color_low >> color_bit) & 0x1;
        let color_high = (color_high >> color_bit) & 0x1;
        let color = (color_high << 1) | color_low;

        let palette = if mem.cgb_mode() { attributes & 0x7 } else { 0 };
        BgPixel { color, palette, bg_over_obj: attributes & 0x80 != 0 }
    }

    fn fetch_obj_pixel(
        &self,
        lx: u8,
        obj_target_y: u8,
        selected_objs: &Vec<usize>,
        objs: &Objs,
        mem: &Memory,
    ) -> Option<ObjPixel> {
        let lcdc = mem[MappedReg::Lcdc];

        if lcdc & 0x2 == 0 {
            // OBJ is disabled
            return None;
        }

        let vram = mem.vram();
        let target = lx + 8;

        let Some(obj) = selected_objs
            .iter()
            .map(|i| &objs[*i as usize])
            .filter(|obj| obj[1] <= target && target < obj[1] + 8)
            .next() else { return None; };

        let y_flip = obj[3] & 0x20 != 0;
        let x_flip = obj[3] & 0x10 != 0;
        let tile_id = if lcdc & 0x4 == 0 {
            // 8x8 mode
            obj[2]
        } else {
            // 8x16 mode
            obj[2] & 0xfe | (((self.ly + 8 > obj[0]) ^ y_flip) as u8)
        };

        let mut y_offset = obj_target_y - obj[0];
        if y_flip {
            y_offset = 7 - y_offset;
        }

        let vram_addr = ((tile_id as usize) << 4) | ((y_offset as usize) << 1);
        let vram_bank = &vram[if mem.cgb_mode() { (obj[3] >> 3) as usize & 0x1 } else { 0 }];
        let color_low = vram_bank[vram_addr];
        let color_high = vram_bank[vram_addr + 1];

        let mut color_bit = target - obj[1];
        if !x_flip {
            color_bit = 7 - color_bit;
        }
        let color_low = (color_low >> color_bit) & 0x1;
        let color_high = (color_high >> color_bit) & 0x1;
        let color = (color_high << 1) | color_low;
        Some(ObjPixel {
            color,
            palette: if mem.cgb_mode() { obj[3] & 0x7 } else { (obj[3] >> 4) & 0x1 },
            bg_over_obj: obj[3] & 0x80 != 0,
        })
    }

    fn mix_pixels(&self, bg_pixel: BgPixel, obj_pixel: Option<ObjPixel>, mem: &Memory) -> u16 {
        let lcdc = mem[MappedReg::Lcdc];
        let bg_palettes = ram_to_palettes(&mem.bg_palette_ram());
        let obj_palettes = ram_to_palettes(&mem.obj_palette_ram());

        let bg_enable_pri = lcdc & 0x1 != 0;
        if let Some(obj_pixel) = obj_pixel {
            let obj_priority = obj_pixel.color != 0
                && if mem.cgb_mode() {
                    bg_pixel.color == 0
                        || !bg_enable_pri
                        || !bg_pixel.bg_over_obj && !obj_pixel.bg_over_obj
                } else {
                    !obj_pixel.bg_over_obj
                };
            if obj_priority {
                let (color, palette) = if mem.cgb_mode() {
                    (obj_pixel.color, obj_pixel.palette)
                } else {
                    let obp = if obj_pixel.palette == 0 {
                        mem[MappedReg::Obp0]
                    } else {
                        mem[MappedReg::Obp1]
                    };
                    ((obp >> (obj_pixel.color * 2)) & 0x3, obj_pixel.palette)
                };

                let palette = obj_palettes[palette as usize];
                return u16::from_le_bytes(palette[color as usize]);
            }
        }

        if !mem.cgb_mode() && !bg_enable_pri {
            // BG disabled; display as white
            return 0x7fff;
        }

        let color = if mem.cgb_mode() {
            bg_pixel.color
        } else {
            let bgp = mem[MappedReg::Bgp];
            (bgp >> (bg_pixel.color * 2)) & 0x3
        };

        let palette = bg_palettes[bg_pixel.palette as usize];
        u16::from_le_bytes(palette[color as usize])
    }

    fn draw_scanline(&self, frame_buff: &mut FrameBuffer, mem: &Memory) {
        let lcdc = mem[MappedReg::Lcdc];

        // OAM Search
        let objs = ram_to_objs(mem.oam());
        let height = (((lcdc >> 2) & 0x1) + 1) * 8; // 8 or 16
        let obj_target_y = self.ly + 16;
        let mut selected_objs: Vec<usize> = objs
            .iter()
            .enumerate()
            .filter(|(_, obj)| obj[0] <= obj_target_y && obj_target_y < obj[0] + height)
            .map(|(i, _)| i)
            .take(10)
            .collect();

        if !mem.cgb_mode() {
            // In compatibility mode, objs with smaller x-coordinate have higher priority. A stable
            // sort is required.
            selected_objs.sort_by_key(|i| objs[*i][1]);
        }

        let scy = mem[MappedReg::Scy];
        let pixel_y = self.ly.wrapping_add(scy);
        let tile_y = pixel_y / 8;
        for lx in 0..Cgb::SCREEN_WIDTH as u8 {
            let obj_pixel = self.fetch_obj_pixel(lx, obj_target_y, &selected_objs, objs, mem);

            let bg_pixel = self.fetch_bg_pixel(lx, pixel_y, tile_y, mem);

            let color = self.mix_pixels(bg_pixel, obj_pixel, mem);

            let mask_rescale = |c| ((c & 0x1f) * 0xff / 0x1f) as u8;
            let red = mask_rescale(color);
            let green = mask_rescale(color >> 5);
            let blue = mask_rescale(color >> 10);
            frame_buff[self.ly as usize][lx as usize] = [red, green, blue, 0xff];
        }
    }

    fn switch_mode(&mut self, mode: ModeState) {
        self.mode_cycles_remaining = mode.cycles();
        self.mode_state = mode;
    }

    fn update_control_regs(&mut self, mem: &mut Memory) {
        mem[MappedReg::Ly] = self.ly;
        mem[MappedReg::Stat] = self.mode_state as u8;
    }

    pub fn execute(&mut self, frame_buff: &mut FrameBuffer, mem: &mut Memory) {
        let lcdc = mem[MappedReg::Lcdc];
        let lcd_enabled = lcdc & 0x80 != 0;

        if !lcd_enabled {
            // TODO: Ideally we would do this only on the first dot after the LCD is disabled.
            self.ly = 0;
            self.mode_state = ModeState::OamSearch;
            self.mode_cycles_remaining = self.mode_state.cycles();
            self.update_control_regs(mem);
            return;
        }

        if self.mode_cycles_remaining > 0 {
            // There are still cycles left for the current mode. Wait to do anything until the last
            // cycle.
            self.mode_cycles_remaining -= 1;
            return;
        }

        match self.mode_state {
            ModeState::OamSearch => self.mode_state = ModeState::Transfer,
            ModeState::Transfer => {
                self.draw_scanline(frame_buff, mem);
                self.switch_mode(ModeState::HBlank);
            }
            ModeState::HBlank => {
                self.ly += 1;
                self.switch_mode(if self.ly == Cgb::SCREEN_HEIGHT as u8 {
                    Interrupt::VBlank.request(mem);
                    ModeState::VBlank
                } else {
                    ModeState::OamSearch
                });
            }
            ModeState::VBlank => {
                self.ly += 1;
                if self.ly == Cgb::FRAME_LINES as u8 {
                    self.ly = 0;
                    self.switch_mode(ModeState::OamSearch);
                } else {
                    self.mode_cycles_remaining = ModeState::VBlank.cycles();
                }
            }
        }

        self.update_control_regs(mem);
    }
}

#[cfg(test)]
mod tests {
    use std::{iter::repeat, mem::MaybeUninit};

    use crate::memory::VRam;

    use super::*;

    struct Context {
        ppu: Ppu,
        mem: Box<Memory>,
        frame_buff: FrameBuffer,
    }

    impl Context {
        fn new(vram_init: impl FnOnce(&mut VRam)) -> Self {
            let mut mem = Memory::new([]);
            vram_init(mem.vram_mut());
            let bg_palette_ram = mem.bg_palette_ram_mut();
            let palette: Vec<u8> = [0xffff, 0x1f << 10, 0x1f << 5, 0x1f]
                .into_iter()
                .flat_map(u16::to_le_bytes)
                .collect();
            bg_palette_ram[0..8].copy_from_slice(&palette);
            mem[MappedReg::Lcdc] = 0x90;
            Self {
                ppu: Ppu::new(&mut mem),
                mem,
                frame_buff: unsafe { MaybeUninit::zeroed().assume_init() },
            }
        }

        fn draw_frame(&mut self) {
            for _ in 0..Cgb::DOTS_PER_FRAME / 4 {
                self.ppu.execute(&mut self.frame_buff, &mut self.mem);
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

    fn checkerboard_vram_init(vram: &mut VRam) {
        vram[0][0..16].copy_from_slice(&[0xff; 16]);
        vram[0][16..32].copy_from_slice(&[0x00; 16]);
        for (y, x) in (0..32).flat_map(|y| repeat(y).zip(0..32)) {
            let addr = 0x1800 + 32 * y + x;
            vram[0][addr] = if x & 0x1 == y & 0x1 { 0x00 } else { 0x01 };
            vram[1][addr] = 0x00;
        }
    }

    #[test]
    fn test_scroll_x() {
        let mut ctx = Context::new(checkerboard_vram_init);
        for scx in 0..=255 {
            ctx.mem[MappedReg::Scx] = scx;
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
    fn test_scroll_y() {
        let mut ctx = Context::new(checkerboard_vram_init);
        for scy in 0..=255 {
            ctx.mem[MappedReg::Scy] = scy;
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
