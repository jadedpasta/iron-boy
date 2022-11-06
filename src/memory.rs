use std::ops::{Index, IndexMut, Range, RangeFrom};

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
}

impl Index<u16> for Memory {
    type Output = u8;

    fn index(&self, addr: u16) -> &Self::Output {
        &self.mem[addr as usize]
    }
}

impl IndexMut<u16> for Memory {
    fn index_mut(&mut self, addr: u16) -> &mut Self::Output {
        &mut self.mem[addr as usize]
    }
}

impl Index<Range<u16>> for Memory {
    type Output = [<Memory as Index<u16>>::Output];

    fn index(&self, index: Range<u16>) -> &Self::Output {
        &self.mem[index.start as usize..index.end as usize]
    }
}

impl IndexMut<Range<u16>> for Memory {
    fn index_mut(&mut self, index: Range<u16>) -> &mut Self::Output {
        &mut self.mem[index.start as usize..index.end as usize]
    }
}

impl Index<RangeFrom<u16>> for Memory {
    type Output = [<Memory as Index<u16>>::Output];

    fn index(&self, index: RangeFrom<u16>) -> &Self::Output {
        &self.mem[index.start as usize..]
    }
}

impl IndexMut<RangeFrom<u16>> for Memory {
    fn index_mut(&mut self, index: RangeFrom<u16>) -> &mut Self::Output {
        &mut self.mem[index.start as usize..]
    }
}

pub trait MemoryView<T> {
    fn read(&self, addr: u16) -> T;
    fn write(&mut self, addr: u16, val: T);
}

impl MemoryView<u8> for Memory {
    fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize] = val;
    }
}

impl MemoryView<u16> for Memory {
    fn read(&self, addr: u16) -> u16 {
        let addr = addr as usize;
        u16::from_le_bytes(self.mem[addr..addr + 2].try_into().unwrap())
    }

    fn write(&mut self, addr: u16, val: u16) {
        let addr = addr as usize;
        self.mem[addr..addr + 2].copy_from_slice(&val.to_le_bytes())
    }
}
