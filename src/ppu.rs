use crate::{memory::{Memory, MappedReg}, Cgb, FrameBuffer, interrupt::Interrupt};

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
    pub fn execute(&mut self, frame_buff: &mut FrameBuffer, mem: &mut Memory) {
        let ly = self.dot / Cgb::DOTS_PER_LINE;
        let pos = self.dot % Cgb::DOTS_PER_LINE;

        let mode = if ly >= Cgb::SCREEN_HEIGHT {
            Mode::VBlank
        } else {
            match pos {
                ..=79 => Mode::OamSearch,
                ..=169 => Mode::Transfer,
                _ => Mode::HBlank,
            }
        };

        mem[MappedReg::Ly] = ly as _;
        mem[MappedReg::Stat] = mode as _;

        if ly == Cgb::SCREEN_HEIGHT {
            Interrupt::VBlank.request(mem);
        }

        self.dot += 1;
        self.dot %= Cgb::DOTS_PER_FRAME;
    }
}
