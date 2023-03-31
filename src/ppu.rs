use crate::{memory::Memory, Cgb, FrameBuffer};

#[derive(Debug, Default)]
pub struct Ppu {
    dot: usize,
}

#[derive(Debug)]
enum Mode {
    HBlank = 0,
    VBlank,
    OamSearch,
    Transfer,
}

impl Ppu {
    const LY_ADDR: u16 = 0xff44;
    const LYC_ADDR: u16 = 0xff45;
    const STAT_ADDR: u16 = 0xff41;

    pub fn execute(&mut self, frame_buff: &mut FrameBuffer, mem: &mut Memory) {
        let ly = self.dot / Cgb::DOTS_PER_LINE;
        let pos = self.dot % Cgb::DOTS_PER_LINE;

        mem.write_8(Self::LY_ADDR, ly as _);

        let mode = if ly >= Cgb::SCREEN_HEIGHT {
            Mode::VBlank
        } else {
            match pos {
                ..=79 => Mode::OamSearch,
                ..=169 => Mode::Transfer,
                _ => Mode::HBlank,
            }
        };

        let stat = mode as _;
        mem.write_8(Self::STAT_ADDR, stat);

        self.dot += 1;
        self.dot %= Cgb::DOTS_PER_FRAME;
    }
}
