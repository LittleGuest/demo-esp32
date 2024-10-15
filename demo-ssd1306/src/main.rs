#![no_std]
#![no_main]

use embedded_graphics::{
    mono_font::{
        ascii::{FONT_6X10, FONT_9X18_BOLD},
        MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Io, Output},
    i2c::I2c,
    prelude::*,
    spi::{slave::Spi, SpiMode},
};
use gc9a01::{prelude::DisplayResolution240x240, Gc9a01};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let delay = Delay::new();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Create a new peripheral object with the described wiring
    // and standard I2C clock speed
    let i2c = I2c::new(peripherals.I2C0, io.pins.gpio4, io.pins.gpio5, 100.kHz());

    // Initialize display
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    // gc9a01
    let sclk = io.pins.gpio2;
    let miso_mosi = io.pins.gpio2;
    let cs = io.pins.gpio10;

    // let miso = miso_mosi.peripheral_input();
    // let mosi = miso_mosi.into_peripheral_output();

    let mut spi = Spi::new(peripherals.SPI2, sclk, mosi, miso, cs, SpiMode::Mode0);

    let dc = Output::new(io.pins.gpio12, esp_hal::gpio::Level::Low);

    let interface = SPIInterface::new(spi, dc);

    let mut display = Gc9a01::new(
        interface,
        DisplayResolution240x240,
        gc9a01::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics();

    // Specify different text styles
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let text_style_big = MonoTextStyleBuilder::new()
        .font(&FONT_9X18_BOLD)
        .text_color(BinaryColor::On)
        .build();

    // Fill display buffer with a centered text with two lines (and two text
    // styles)
    Text::with_alignment(
        "esp-hal",
        display.bounding_box().center() + Point::new(0, 0),
        text_style_big,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    Text::with_alignment(
        "Chip: ESP32S3",
        display.bounding_box().center() + Point::new(0, 14),
        text_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    // Write buffer to display
    display.flush().unwrap();
    // Clear display buffer
    display.clear(BinaryColor::Off).unwrap();

    loop {
        delay.delay_millis(1);
    }
}
