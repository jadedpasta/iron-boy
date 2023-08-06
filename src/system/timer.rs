use super::CgbSystem;
use crate::{interrupt::Interrupt, timer::TimerBus};
use partial_borrow::prelude::*;

impl TimerBus for partial!(CgbSystem ! timer, mut mem interrupt) {
    fn request_timer_interrupt(&mut self) {
        self.interrupt.request(Interrupt::Timer);
    }
}
