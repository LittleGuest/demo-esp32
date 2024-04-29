use esp_idf_hal::{
    adc::{self, AdcDriver},
    delay::FreeRtos,
    prelude::Peripherals,
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();

    let mut adc1 = AdcDriver::new(
        peripherals.adc1,
        &adc::config::Config::new().calibration(true),
    )
    .unwrap();

    let mut a1_ch0 =
        adc::AdcChannelDriver::<_, adc::Atten11dB<adc::ADC1>>::new(peripherals.pins.gpio0).unwrap();

    loop {
        match adc1.read(&mut a1_ch0) {
            Ok(x) => {
                println!("A1_CH0: {x}")
            }
            Err(e) => {
                println!("error reading ADC: {e:?}");
            }
        }

        FreeRtos::delay_ms(1000);
    }
}
