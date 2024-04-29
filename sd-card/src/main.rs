use embedded_sdmmc::*;
use esp_idf_hal::{
    gpio::*,
    prelude::*,
    spi::{config::Duplex, *},
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

struct SdMmcClock;

impl TimeSource for SdMmcClock {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();

    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio8,
        peripherals.pins.gpio7,
        Some(peripherals.pins.gpio9),
        Dma::Disabled,
    )
    .unwrap();

    let mut spi_config = SpiConfig::new();
    spi_config.duplex = Duplex::Full;
    let _ = spi_config.baudrate(24.MHz().into());
    let spi = SpiDeviceDriver::new(driver, Option::<Gpio6>::None, &spi_config).unwrap();

    let cs = PinDriver::output(peripherals.pins.gpio6).unwrap();
    let mut sdmmc_spi = embedded_sdmmc::SdMmcSpi::new(spi, cs);

    match sdmmc_spi.acquire() {
        Ok(block) => {
            let mut controller = embedded_sdmmc::Controller::new(block, SdMmcClock);
            let mut volume = controller.get_volume(embedded_sdmmc::VolumeIdx(0)).unwrap();

            let root_dir = controller.open_root_dir(&volume).unwrap();
            let mut f = controller
                .open_file_in_dir(
                    &mut volume,
                    &root_dir,
                    "demo-esp32c3-rust-sd-card.txt",
                    embedded_sdmmc::Mode::ReadWriteCreateOrAppend,
                )
                .unwrap();

            f.seek_from_end(0).unwrap();

            let log_string = "Hello SD card!".to_string();
            let _ = controller.write(&mut volume, &mut f, &log_string.as_bytes());

            let _ = controller.close_file(&volume, f);
        }
        Err(e) => {
            println!("{e:?}");
        }
    };
}
