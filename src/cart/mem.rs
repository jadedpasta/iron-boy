// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

pub struct Segment(Box<[u8]>);

impl Segment {
    pub fn new(len: usize) -> Self {
        assert!(len.is_power_of_two());
        Self((0..len).map(|_| 0).collect())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn read(&self, addr: usize) -> u8 {
        self.0[addr & (self.len() - 1)]
    }

    pub fn write(&mut self, addr: usize, val: u8) {
        self.0[addr & (self.len() - 1)] = val;
    }
}

impl TryFrom<Box<[u8]>> for Segment {
    type Error = (); // TODO: better error

    fn try_from(buf: Box<[u8]>) -> Result<Self, Self::Error> {
        if buf.len().is_power_of_two() {
            Ok(Self(buf))
        } else {
            Err(())
        }
    }
}

pub struct OptionalSegment(Option<Segment>);

impl OptionalSegment {
    pub fn new(len: usize) -> Self {
        Self(if len == 0 { None } else { Some(Segment::new(len)) })
    }

    pub fn len(&self) -> usize {
        if let Self(Some(segment)) = self {
            segment.len()
        } else {
            0
        }
    }

    pub fn read(&self, addr: usize) -> u8 {
        if let Self(Some(segment)) = self {
            segment.read(addr)
        } else {
            0xff
        }
    }

    pub fn write(&mut self, addr: usize, val: u8) {
        if let Self(Some(segment)) = self {
            segment.write(addr, val);
        }
    }
}

impl TryFrom<Box<[u8]>> for OptionalSegment {
    type Error = (); // TODO: better error

    fn try_from(buf: Box<[u8]>) -> Result<Self, Self::Error> {
        Ok(Self(if buf.len() == 0 { None } else { Some(buf.try_into()?) }))
    }
}

pub struct Mem {
    pub rom: Segment,
    pub ram: OptionalSegment,
}
