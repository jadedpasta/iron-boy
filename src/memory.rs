pub struct Memory {
    mem: Box<[u8; 0x10000]>,
}

impl Memory {
    pub fn new(mem: impl Into<Vec<u8>>) -> Self {
        let mut mem = mem.into();
        mem.resize(0x10000, 0);
        Self {
            mem: mem.into_boxed_slice().try_into().unwrap(),
        }
    }

    pub fn read_8(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    pub fn write_8(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize] = val
    }

    pub fn read_16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.mem[addr as usize], self.mem[addr.wrapping_add(1) as usize]])
    }

    pub fn write_16(&mut self, addr: u16, val: u16) {
        [self.mem[addr as usize], self.mem[addr.wrapping_add(1) as usize]] = val.to_le_bytes();
    }
}
