use std::num::Wrapping;

pub trait TimerBus {
    fn request_timer_interrupt(&mut self);
}

pub struct Timer {
    counter: Wrapping<u16>,
    tima: Wrapping<u8>,
    tma: u8,
    tac: u8,
}

const ENABLE: u8 = 0x4;

impl Timer {
    pub fn new() -> Self {
        Timer { counter: Wrapping(0), tima: Wrapping(0), tma: 0, tac: 0 }
    }

    pub fn execute(&mut self, bus: &mut impl TimerBus) {
        let old_counter = self.counter;
        // In real hardware, this counter increments once per T-cycle, but we only call this once
        // per M-cycle.
        self.counter += 4;

        if self.tac & ENABLE == 0 {
            // Timer is disabled
            return;
        }

        // Increase TIMA at the TAC-configured frequency
        // 00 -> clock / 2^10
        // 01 -> clock / 2^4
        // 10 -> clock / 2^6
        // 10 -> clock / 2^8
        let turned_off = old_counter.0 & !self.counter.0;
        let freq = self.tac.wrapping_sub(1) & 0x3;
        if turned_off >> (2 * freq + 3) != 0 {
            self.tima += 1;
            if self.tima.0 == 0 {
                // overflow
                self.tima.0 = self.tma;
                bus.request_timer_interrupt();
            }
        }
    }

    pub fn div(&self) -> u8 {
        (self.counter.0 >> 8) as u8
    }

    pub fn reset_div(&mut self) {
        self.counter.0 = 0;
    }

    pub fn tima(&self) -> u8 {
        self.tima.0
    }

    pub fn set_tima(&mut self, tima: u8) {
        self.tima.0 = tima;
    }

    pub fn tma(&self) -> u8 {
        self.tma
    }

    pub fn set_tma(&mut self, tma: u8) {
        self.tma = tma;
    }

    pub fn tac(&self) -> u8 {
        self.tac
    }

    pub fn set_tac(&mut self, tac: u8) {
        self.tac = tac;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct InterruptModerator<F> {
        func: F,
    }

    impl<F> TimerBus for InterruptModerator<F>
    where
        F: FnMut() -> (),
    {
        fn request_timer_interrupt(&mut self) {
            (self.func)();
        }
    }

    fn tma_ff(tac: u8, period: u32) {
        let period = period / 4;

        let mut timer = Timer::new();
        timer.set_tma(0xff);
        timer.set_tima(0xff);
        timer.set_tac(tac | ENABLE);

        let mut requests = 0;
        for i in 0..10 * period {
            let mut bus = InterruptModerator {
                func: || {
                    requests += 1;
                    // Falling edge, so we increment at the end of the cycle
                    assert!((i + 1) % period == 0, "Requested interrupt when not expected");
                },
            };
            timer.execute(&mut bus);
        }
        assert_eq!(requests, 10, "Did not request correct amount of interrupts");
    }

    #[test]
    fn tma_ff_00() {
        tma_ff(0b00, 1 << 10);
    }

    #[test]
    fn tma_ff_01() {
        tma_ff(0b01, 1 << 4);
    }

    #[test]
    fn tma_ff_10() {
        tma_ff(0b10, 1 << 6);
    }

    #[test]
    fn tma_ff_11() {
        tma_ff(0b11, 1 << 8);
    }
}
