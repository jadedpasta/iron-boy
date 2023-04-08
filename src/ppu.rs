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

#[derive(Debug)]
enum Mode {
    HBlank = 0,
    VBlank,
    OamSearch,
    Transfer,
}

#[derive(Debug, Default)]
pub struct Ppu {
    fetcher: PixelFetcher,
    bg_fifo: PixelFifo,
    dot: usize,
    lx: u8,
    skip_count: u8,
    skip_set: bool,
}

type Color = [u8; 2];
type Palette = [Color; 4];
type Palettes = [Palette; 8];

fn ram_to_palettes(ram: &PaletteRam) -> &Palettes {
    unsafe { mem::transmute(ram) }
}

impl Ppu {
    fn draw(&mut self, ly: usize, frame_buff: &mut FrameBuffer, mem: &mut Memory) {
        self.fetcher.fetch(self.lx, &mut self.bg_fifo, mem);

        let scx = mem[MappedReg::Scx];

        if self.lx == 0 && self.skip_count == 0 && !self.skip_set {
            self.skip_set = true;
            self.skip_count = scx & 0x7;
        }

        if let Some(pixel) = self.bg_fifo.pop() {
            if self.skip_count > 0 {
                self.skip_count -= 1;
            } else {
                let palette = ram_to_palettes(&mem.bg_palette_ram())[pixel.palette as usize];
                let color = u16::from_le_bytes(palette[pixel.color as usize]);
                let mask_rescale = |c| ((c & 0x1f) * 0xff / 0x1f) as u8;
                let red = mask_rescale(color);
                let green = mask_rescale(color >> 5);
                let blue = mask_rescale(color >> 10);
                frame_buff[ly][self.lx as usize] = [red, green, blue, 0xff];
                self.lx += 1;
            }
        }

        if self.lx >= Cgb::SCREEN_WIDTH as u8 {
            self.bg_fifo.size = 0;
            self.skip_set = false;
        }
    }

    pub fn execute(&mut self, frame_buff: &mut FrameBuffer, mem: &mut Memory) {
        let ly = self.dot / Cgb::DOTS_PER_LINE;
        let pos = self.dot % Cgb::DOTS_PER_LINE;

        mem[MappedReg::Ly] = ly as _;

        let lcdc = mem[MappedReg::Lcdc];
        let lcd_enabled = lcdc & 0x80 != 0;
        if lcd_enabled {
            let mode = if ly >= Cgb::SCREEN_HEIGHT {
                Mode::VBlank
            } else if self.lx >= Cgb::SCREEN_WIDTH as u8 && pos > 80 {
                Mode::HBlank
            } else if pos < 80 {
                // TODO: OAM Search
                Mode::OamSearch
            } else {
                if pos == 80 {
                    self.lx = 0;
                }
                self.draw(ly, frame_buff, mem);
                Mode::Transfer
            };

            if ly == Cgb::SCREEN_HEIGHT {
                Interrupt::VBlank.request(mem);
            }

            mem[MappedReg::Stat] = mode as _;
        }

        self.dot += 1;
        self.dot %= Cgb::DOTS_PER_FRAME;
    }
}
