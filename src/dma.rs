use crate::system::{CgbSystem, MappedReg};

pub enum DmaType {
    Oam,
    General,
}

pub struct DmaState {
    pub ty: DmaType,
    pub len: u16,
    pub count: u16,
    pub oam_src: u16,
}

pub struct Dma;

impl Dma {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&mut self, mem: &mut CgbSystem) {
        let Some(state) = &mem.dma_state else { return; };

        match state.ty {
            DmaType::General => {
                // Ensure the CPU is stalled during the transfer
                mem.cpu_dma_paused = true;
                // Copy 2 bytes per M-cycle
                let hdma1 = mem[MappedReg::Hdma1] as u16;
                let hdma2 = mem[MappedReg::Hdma2] as u16;
                let hdma3 = mem[MappedReg::Hdma3] as u16;
                let hdma4 = mem[MappedReg::Hdma4] as u16;
                let vbk = mem[MappedReg::Vbk] as usize & 0x1;
                let src_addr = state.count.wrapping_add(((hdma1 << 8) | hdma2) & 0xfff0);
                let dst_addr = state.count.wrapping_add(((hdma3 << 8) | hdma4) & 0x1ff0) & 0x1fff;
                mem.vram_mut()[vbk][dst_addr as usize] = mem.read_8(src_addr);
                let src_addr = src_addr.wrapping_add(1);
                let dst_addr = (dst_addr + 1) & 0x1fff;
                mem.vram_mut()[vbk][dst_addr as usize] = mem.read_8(src_addr);
            }
            DmaType::Oam => {
                let src_addr = state.oam_src.wrapping_add(state.count);
                let dst_addr = state.count;
                mem.oam_mut()[dst_addr as usize] = mem.read_8(src_addr);
            }
        }

        // Gotta reborrow to keep the borrow checker happy. Hopefully this can be optimized out?
        let state = mem.dma_state.as_mut().unwrap();

        state.count += match state.ty {
            DmaType::General => 2,
            DmaType::Oam => 1,
        };

        if state.count == state.len {
            // Transfer is complete
            mem.dma_state = None;
            mem.cpu_dma_paused = false;
        }
    }
}
