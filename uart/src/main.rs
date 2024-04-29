use esp_idf_hal::{
    delay::{FreeRtos, NON_BLOCK},
    gpio,
    prelude::Peripherals,
    uart::{self, UartDriver},
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let tx = peripherals.pins.gpio21;
    let rx = peripherals.pins.gpio20;

    let config = uart::config::Config::new().baudrate(esp_idf_hal::units::Hertz(115_200));

    let uart = UartDriver::new(
        peripherals.uart1,
        tx,
        rx,
        Option::<gpio::Gpio0>::None,
        Option::<gpio::Gpio1>::None,
        &config,
    )
    .unwrap();

    let mut cli_buf: Vec<u8> = Vec::new();

    loop {
        let mut buf: [u8; 10] = [0; 10];
        match uart.read(&mut buf, NON_BLOCK) {
            Ok(num_bytes) => {
                if num_bytes > 0 {
                    let b = buf[0];
                    cli_buf.push(b);
                    if b == 13 {
                        match uart.write(&cli_buf) {
                            Ok(_) => {
                                println!("{cli_buf:?}");
                            }
                            Err(_) => {}
                        }
                        cli_buf.clear();
                    }
                }
            }
            Err(_) => {}
        }

        FreeRtos::delay_ms(100);
    }
}
