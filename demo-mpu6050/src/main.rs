use std::sync::{Arc, Mutex};

use embedded_svc::http::Method;
use esp_idf_hal::sys::EspError;
use esp_idf_svc::{
    hal::{delay::FreeRtos, i2c::I2cDriver, prelude::Peripherals},
    wifi::{ClientConfiguration, Configuration},
};
use log::info;
use mpu6050_dmp::{accel::AccelFullScale, address::Address, gyro::GyroFullScale, sensor::Mpu6050};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::EspHttpServer,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};

static INDEX_HTML: &str = include_str!("../index.html");
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");

struct Delay;

impl embedded_hal::blocking::delay::DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        FreeRtos::delay_ms(ms as u32);
    }
}

impl embedded_hal::blocking::delay::DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        FreeRtos::delay_ms(ms);
    }
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let i2c_driver = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio4,
        peripherals.pins.gpio5,
        &Default::default(),
    )?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
    )?;

    let config = Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.into(),
        password: WIFI_PASS.into(),
        ..Default::default()
    });
    wifi.set_configuration(&config)?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    info!(
        "获取到的IP地址为:{:?}",
        wifi.wifi().sta_netif().get_ip_info()
    );

    std::mem::forget(wifi);

    let mut delay = Delay;
    let mut mpu = Mpu6050::new(i2c_driver, Address::default()).unwrap();
    mpu.initialize_dmp(&mut delay).unwrap();

    let mpu = Arc::new(Mutex::new(mpu));

    let mpu = mpu.clone();
    let mut server = EspHttpServer::new(&Default::default())?;
    server
        .fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?.write(INDEX_HTML.as_bytes())?;
            Ok(())
        })?
        .ws_handler("/ws/mpu6050", move |ws| {
            if ws.is_new() {
                info!("ws is new");
            }
            if ws.is_closed() {
                info!("ws is closed");
            }

            info!("ws send data");
            loop {
                let gyro = mpu
                    .lock()
                    .unwrap()
                    .gyro()
                    .unwrap()
                    .scaled(GyroFullScale::Deg2000);
                let gx = gyro.x() / 57.29577;
                let gy = gyro.y() / 57.29577;
                let gz = gyro.z() / 57.29577;

                let acc = mpu
                    .lock()
                    .unwrap()
                    .accel()
                    .unwrap()
                    .scaled(AccelFullScale::G2);
                let ax = acc.x();
                let ay = acc.y();
                let az = acc.z();

                let data = format!("{gx},{gy},{gz},{ax},{ay},{az},24");
                if ws
                    .send(embedded_svc::ws::FrameType::Text(false), data.as_bytes())
                    .is_err()
                {
                    break;
                };
                FreeRtos::delay_us(100);
            }

            Ok::<(), EspError>(())
        })?;
    std::mem::forget(server);

    Ok(())
}
