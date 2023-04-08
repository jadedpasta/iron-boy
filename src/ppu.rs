use std::mem;

use crate::{
    interrupt::Interrupt,
    memory::{MappedReg, Memory, PaletteRam},
    Cgb, FrameBuffer,
};

#[derive(Debug, Default, Clone, Copy)]
struct Pixel {
    color: u8,
    palette: u8,
    obj: Option<u8>,
    priority: u8,
}

#[derive(Debug, Default)]
enum PixelFetcherState {
    #[default]
    ComputeTileAddress,
    FetchTileId {
        vram_addr: u16,
    },
    ComputeAddress {
        high: bool,
    },
    FetchTile {
        high: bool,
        vram_addr: u16,
    },
    Push,
}

#[derive(Debug, Default)]
struct PixelFetcher {
    state: PixelFetcherState,
    tile_id: u8,
    attributes: u8,
    pixel_data: [u8; 2],
}

impl PixelFetcher {
    fn fetch(&mut self, lx: u8, fifo: &mut PixelFifo, mem: &Memory) {
        // We may have pixels already in the FIFO. Fetch the pixels that come next, not at the
        // location of the current pixel to be shifted out.
        let pixel_x = lx + fifo.size as u8;
        use PixelFetcherState::*;
        match self.state {
            ComputeTileAddress => {
                let lcdc = mem[MappedReg::Lcdc];
                let ly = mem[MappedReg::Ly];
                let scy = mem[MappedReg::Scy];
                let scx = mem[MappedReg::Scx];
                let map_area_bit = (lcdc >> 3) & 0x1;
                let y = ly.wrapping_add(scy) / 8;
                let x = pixel_x.wrapping_add(scx) / 8;
                let vram_addr =
                    0x1800 | ((map_area_bit as u16) << 10) | ((y as u16) << 5) | x as u16;
                self.state = FetchTileId { vram_addr }
            }
            FetchTileId { vram_addr } => {
                let vram = mem.vram();
                self.tile_id = vram[0][vram_addr as usize];
                self.attributes = vram[1][vram_addr as usize];
                self.state = ComputeAddress { high: false };
            }
            ComputeAddress { high } => {
                let lcdc = mem[MappedReg::Lcdc];
                let ly = mem[MappedReg::Ly];
                let scy = mem[MappedReg::Scy];
                let y_offset = ly.wrapping_add(scy) & 0x7;
                let addr_mode_bit = !((lcdc >> 4) | (self.tile_id >> 7)) & 0x1;
                let vram_addr = ((addr_mode_bit as u16) << 12)
                    | ((self.tile_id as u16) << 4)
                    | ((y_offset as u16) << 1)
                    | high as u16;
                self.state = FetchTile { high, vram_addr };
            }
            FetchTile { high, vram_addr } => {
                let bank = (self.attributes >> 3) & 0x1;
                self.pixel_data[high as usize] = mem.vram()[bank as usize][vram_addr as usize];
                self.state = if high { Push } else { ComputeAddress { high: true } };
            }
            Push if fifo.size == 0 => {
                let color_low = self.pixel_data[0];
                let color_high = self.pixel_data[1];
                let palette = self.attributes & 0x7;
                let priority = self.attributes >> 7;
                for (mut i, pixel) in fifo.fifo.iter_mut().enumerate() {
                    i = 7 - i;
                    let color_low = (color_low >> i) & 0x1;
                    let color_high = (color_high >> i) & 0x1;
                    *pixel = Pixel {
                        color: (color_high << 1) | color_low,
                        palette,
                        obj: None,
                        priority,
                    };
                }
                fifo.size = 8;
                self.state = ComputeTileAddress;
            }
            Push => (),
        }
    }

    fn reset(&mut self) {
        self.state = Default::default();
    }
}

#[derive(Debug, Default)]
struct PixelFifo {
    fifo: [Pixel; 8],
    size: usize,
}

impl PixelFifo {
    fn pop(&mut self) -> Option<Pixel> {
        if self.size > 0 {
            let pixel = self.fifo[self.fifo.len() - self.size];
            self.size -= 1;
            Some(pixel)
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
struct PixelPipeline {
    fetcher: PixelFetcher,
    bg_fifo: PixelFifo,
}

impl PixelPipeline {
    fn try_fetch_pixel(&mut self, lx: u8, mem: &Memory) -> Option<Pixel> {
        self.fetcher.fetch(lx, &mut self.bg_fifo, mem);
        self.bg_fifo.pop()
    }

    fn reset(&mut self) {
        self.fetcher.reset();
        self.bg_fifo.size = 0;
    }
}

#[derive(Debug)]
enum DrawState {
    Scroll { skip_count: u8 },
    Draw { lx: u8 },
}

impl DrawState {
    fn lx(&self) -> u8 {
        if let Self::Draw { lx } = self {
            *lx
        } else {
            0
        }
    }
}

#[derive(Debug, Default)]
#[repr(u8)]
enum ModeState {
    HBlank = 0,
    VBlank,
    #[default]
    OamSearch,
    Transfer(DrawState),
}

impl ModeState {
    fn mode(&self) -> u8 {
        // SAFETY: This is safe with repr(..) enums. Discriminant is always stored at the beginning
        // of the allocation in this case.
        unsafe { *(self as *const _ as *const _) }
    }

    fn start_transfer(&mut self, mem: &Memory) {
        let scx = mem[MappedReg::Scx];
        let skip_count = scx & 0x7;
        let draw_state = if skip_count > 0 {
            DrawState::Scroll { skip_count }
        } else {
            DrawState::Draw { lx: 0 }
        };
        *self = Self::Transfer(draw_state);
    }
}

#[derive(Debug)]
pub struct Ppu {
    mode_state: ModeState,
    pipeline: PixelPipeline,
    ly: u8,
    line_dot: usize,
}

type Color = [u8; 2];
type Palette = [Color; 4];
type Palettes = [Palette; 8];

fn ram_to_palettes(ram: &PaletteRam) -> &Palettes {
    unsafe { mem::transmute(ram) }
}

impl Ppu {
    pub fn new(mem: &mut Memory) -> Self {
        let mut result = Self {
            mode_state: Default::default(),
            pipeline: Default::default(),
            ly: 0,
            line_dot: 0,
        };
        // Registers should be given initial values on startup. Not sure how actual hardware
        // behaves, but this is nice for an emulator.
        result.update_control_regs(mem);
        result
    }

    fn draw(
        ly: u8,
        draw_state: &mut DrawState,
        pipeline: &mut PixelPipeline,
        frame_buff: &mut FrameBuffer,
        mem: &Memory,
    ) -> bool {
        let Some(pixel) = pipeline.try_fetch_pixel(draw_state.lx(), mem) else { return false };
        match draw_state {
            DrawState::Scroll { skip_count } => {
                *skip_count -= 1;
                if *skip_count == 0 {
                    *draw_state = DrawState::Draw { lx: 0 };
                }
                false
            }
            DrawState::Draw { lx } => {
                let palette = ram_to_palettes(&mem.bg_palette_ram())[pixel.palette as usize];
                let color = u16::from_le_bytes(palette[pixel.color as usize]);
                let mask_rescale = |c| ((c & 0x1f) * 0xff / 0x1f) as u8;
                let red = mask_rescale(color);
                let green = mask_rescale(color >> 5);
                let blue = mask_rescale(color >> 10);
                frame_buff[ly as usize][*lx as usize] = [red, green, blue, 0xff];
                *lx += 1;
                *lx == Cgb::SCREEN_WIDTH as u8
            }
        }
    }

    fn start_transfer(&mut self, mem: &mut Memory) {
        self.pipeline.reset();
        self.mode_state.start_transfer(mem);
    }

    fn update_control_regs(&mut self, mem: &mut Memory) {
        mem[MappedReg::Ly] = self.ly;
        mem[MappedReg::Stat] = self.mode_state.mode();
    }

    pub fn execute(&mut self, frame_buff: &mut FrameBuffer, mem: &mut Memory) {
        let lcdc = mem[MappedReg::Lcdc];
        let lcd_enabled = lcdc & 0x80 != 0;

        if !lcd_enabled {
            // TODO: Ideally we would do this only on the first dot after the LCD is disabled.
            self.ly = 0;
            self.line_dot = 0;
            self.mode_state = ModeState::OamSearch;
            self.pipeline.reset();
            self.update_control_regs(mem);
            return;
        }

        match &mut self.mode_state {
            ModeState::OamSearch => {
                // TODO: OAM Search
                self.line_dot += 1;
                if self.line_dot == 80 {
                    self.start_transfer(mem);
                }
            }
            ModeState::Transfer(draw_state) => {
                let scanline_completed =
                    Self::draw(self.ly, draw_state, &mut self.pipeline, frame_buff, mem);
                self.line_dot += 1;
                // Assuming that we will always have at least 1 dot of HBlank, no matter what is
                // drawn to the screen.
                assert!(self.line_dot < Cgb::DOTS_PER_LINE);
                if scanline_completed {
                    self.mode_state = ModeState::HBlank;
                }
            }
            blank @ (ModeState::HBlank | ModeState::VBlank) => {
                self.line_dot += 1;
                self.line_dot %= Cgb::DOTS_PER_LINE;
                if self.line_dot == 0 {
                    self.ly += 1;
                    self.ly %= (Cgb::SCREEN_HEIGHT + Cgb::VBLANK_LINES) as u8;

                    if let ModeState::HBlank = blank {
                        self.mode_state = ModeState::OamSearch;
                    }
                    if self.ly == Cgb::SCREEN_HEIGHT as u8 {
                        self.mode_state = ModeState::VBlank;
                        Interrupt::VBlank.request(mem);
                    } else if self.ly == 0 {
                        self.mode_state = ModeState::OamSearch;
                    }
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
            for _ in 0..Cgb::DOTS_PER_FRAME {
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
