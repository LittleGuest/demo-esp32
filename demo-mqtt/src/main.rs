#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::mem::MaybeUninit;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{dns::DnsQueryType, Config, Ipv4Address, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{GpioPin, OpenDrain, Output, IO};
use esp_hal::prelude::{entry, main};
use esp_hal::rng::Rng;
use esp_hal::systimer::SystemTimer;
use esp_hal::timer::TimerGroup;
use esp_hal::{clock::ClockControl, peripherals::Peripherals, prelude::*};
use esp_println::println;
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
    WifiState,
};
use esp_wifi::EspWifiInitFor;
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::{ClientConfig, MqttVersion};
use rust_mqtt::packet::v5::publish_packet::QualityOfService;
use rust_mqtt::utils::rng_generator::CountingRng;
use serde::{Deserialize, Serialize};
use static_cell::make_static;

mod dht;

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

pub fn init() {
    const HEAP_SIZE: usize = 64 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

/// temperature and humidity
#[derive(Serialize, Deserialize)]
struct TempHumi {
    #[serde(rename = "d")]
    date: u32,
    #[serde(rename = "t")]
    temperature: f32,
    #[serde(rename = "h")]
    humidity: f32,
}

#[main]
async fn main(spawner: Spawner) {
    init();
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let gpio2 = io.pins.gpio2.into_open_drain_output();

    let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = esp_wifi::initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let tg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);

    log::info!("创建网络接口");
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, peripherals.WIFI, WifiStaDevice).unwrap();

    esp_hal::embassy::init(&clocks, tg0);

    log::info!("配置WIFI");
    let config = Config::dhcpv4(Default::default());

    let seed = 1234;
    let stack = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<3>::new()),
        seed,
    ));

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(stack)).ok();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("等待获取IP...");
    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("获取到的IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    spawner.spawn(publish_msg(stack, gpio2)).ok();
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    log::info!("开始connection task");
    log::info!("设备能力: {:?}", controller.get_capabilities());
    loop {
        if WifiState::StaConnected == esp_wifi::wifi::get_wifi_state() {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: WIFI_SSID.try_into().unwrap(),
                password: WIFI_PASS.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            log::info!("开始启动WIFI");
            controller.start().await.unwrap();
            log::info!("WIFI启动成功");
        }
        log::info!("开始连接WIFI");

        match controller.connect().await {
            Ok(_) => log::info!("WIFI连接成功"),
            Err(e) => {
                log::error!("WIFI连接失败: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await;
}

#[embassy_executor::task]
async fn publish_msg(
    stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>,
    pin: GpioPin<Output<OpenDrain>, 2>,
) {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

        let address = match stack
            .dns_query("broker.emqx.io", DnsQueryType::A)
            .await
            .map(|a| a[0])
        {
            Ok(address) => address,
            Err(e) => {
                println!("DNS lookup error: {e:?}");
                continue;
            }
        };
        // let remote_endpoint = (Ipv4Address::new(192, 168, 31, 125), 1884);
        let remote_endpoint = (address, 1883);
        let connection = socket.connect(remote_endpoint).await;
        if let Err(e) = connection {
            log::error!("mqtt server 连接失败 : {e:?}");
            Timer::after_secs(5).await;
            continue;
        }
        log::info!("mqtt server 连接成功");

        let mut config = ClientConfig::new(MqttVersion::MQTTv5, CountingRng(20000));
        config.add_max_subscribe_qos(QualityOfService::QoS1);
        config.add_client_id("demo-mqtt-no-std");
        config.max_packet_size = 100;
        // config.add_username(USERNAME);
        // config.add_password(PASSWORD);

        let mut recv_buffer = [0; 80];
        let mut write_buffer = [0; 80];

        let mut client =
            MqttClient::<_, 5, _>::new(socket, &mut write_buffer, 80, &mut recv_buffer, 80, config);
        client.connect_to_broker().await.ok();

        let mut dht = dht::Dht11::new(pin);
        loop {
            match dht.perform_measurement(&mut embassy_time::Delay) {
                Ok(r) => {
                    log::info!(
                        "humidity: {}, temperature: {}",
                        r.humidity as f32 / 10.0,
                        r.temperature as f32 / 10.0,
                    );
                    let th = TempHumi {
                        date: 0,
                        temperature: r.temperature as f32 / 10.0,
                        humidity: r.humidity as f32 / 10.0,
                    };

                    client
                        .send_message(
                            "testtopic/pjq/dht11",
                            serde_json::to_string(&th).unwrap().as_bytes(),
                            QualityOfService::QoS1,
                            false,
                        )
                        .await
                        .ok();
                }
                Err(e) => {
                    log::error!("dht read err: {e:?}");
                }
            }

            Timer::after_secs(5).await;
        }
    }
}
