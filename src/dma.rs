use crate::system::{Oam, VRam};

pub enum DmaType {
    Oam,
    General,
}

struct DmaState {
    pub ty: DmaType,
    pub len: u16,
    pub count: u16,
    pub oam_src: u16,
}

pub trait DmaBus {
    fn general_src_addr(&self) -> u16;
    fn general_dst_addr(&self) -> u16;
    fn vbk(&self) -> usize;
    fn vram_mut(&mut self) -> &mut VRam;
    fn oam_mut(&mut self) -> &mut Oam;
    fn read_8(&self, addr: u16) -> u8;
}

pub struct Dma {
    state: Option<DmaState>,
    cpu_paused: bool,
}

impl Dma {
    pub fn new() -> Self {
        Self { state: None, cpu_paused: false }
    }

    pub fn cpu_paused(&self) -> bool {
        self.cpu_paused
    }

    pub fn start_general(&mut self, len: u16) {
        // TODO: Do some kind of cancel of an ongoing OAM DMA for simplicity
        self.state = Some(DmaState { ty: DmaType::General, len, count: 0, oam_src: 0 });
    }

    pub fn start_oam(&mut self, oam_src: u16) {
        // TODO: Do some kind of cancel of an ongoing HDMA for simplicity
        self.state = Some(DmaState { ty: DmaType::Oam, len: 0xa0, count: 0, oam_src });
    }

    pub fn execute(&mut self, bus: &mut impl DmaBus) {
        let Some(state) = &self.state else { return; };

        match state.ty {
            DmaType::General => {
                // Ensure the CPU is stalled during the transfer
                self.cpu_paused = true;
                // Copy 2 bytes per M-cycle
                let vbk = bus.vbk();
                let src_addr = bus.general_src_addr().wrapping_add(state.count);
                let dst_addr = bus.general_dst_addr().wrapping_add(state.count) & 0x1fff;
                bus.vram_mut()[vbk][dst_addr as usize] = bus.read_8(src_addr);
                let src_addr = src_addr.wrapping_add(1);
                let dst_addr = (dst_addr + 1) & 0x1fff;
                bus.vram_mut()[vbk][dst_addr as usize] = bus.read_8(src_addr);
            }
            DmaType::Oam => {
                let src_addr = state.oam_src.wrapping_add(state.count);
                let dst_addr = state.count;
                bus.oam_mut()[dst_addr as usize] = bus.read_8(src_addr);
            }
        }

        // Gotta reborrow to keep the borrow checker happy. Hopefully this can be optimized out?
        let state = self.state.as_mut().unwrap();

        state.count += match state.ty {
            DmaType::General => 2,
            DmaType::Oam => 1,
        };

        if state.count == state.len {
            // Transfer is complete
            self.state = None;
            self.cpu_paused = false;
        }
    }
}
