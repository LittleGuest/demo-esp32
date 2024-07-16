#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{clock::ClockControl, delay::Delay, gpio::IO, peripherals::Peripherals, prelude::*};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let delay = Delay::new(&clocks);
    esp_println::logger::init_logger_from_env();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let mut led5 = io.pins.gpio13.into_push_pull_output();

    loop {
        led5.toggle();
        delay.delay(500.millis());
    }
}
