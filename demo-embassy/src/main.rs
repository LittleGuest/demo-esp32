#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl, delay::Delay, embassy, peripherals::Peripherals, prelude::*,
    timer::TimerGroup,
};
use log::info;

extern crate alloc;
use core::mem::MaybeUninit;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    init_heap();
    esp_println::logger::init_logger_from_env();
    let tg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    esp_hal::embassy::init(&clocks, tg0);

    let mut count = 0;

    loop {
        count += 1;
        spawner.spawn(test(count)).ok();
        log::info!("Hello world!");
        Timer::after_millis(100).await;
    }
}

#[embassy_executor::task]
async fn test(count: i32) {
    info!("print test once, {count}");
    Timer::after_millis(500).await;
    info!("print test once after, {count}");
}
