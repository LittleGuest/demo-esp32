#![no_std]
#![no_main]

use embedded_io::{Read, Write};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Output, IO},
    peripherals::Peripherals,
    prelude::*,
};
use esp_wifi::{
    current_millis,
    wifi::{
        utils::create_network_interface, AccessPointInfo, ClientConfiguration, Configuration,
        WifiStaDevice,
    },
    wifi_interface::WifiStack,
};
use log::{error, info};
use reqwless::client::HttpClient;
use smoltcp::{
    iface::SocketStorage,
    wire::{IpAddress, Ipv4Address},
};

use core::mem::MaybeUninit;

mod ota;

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");
const HOST_IP: &str = env!("HOST_IP");
const PORT: u16 = 8080;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let delay = Delay::new(&clocks);
    init_heap();
    esp_println::logger::init_logger_from_env();
    let timer = esp_hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = esp_wifi::initialize(
        esp_wifi::EspWifiInitFor::Wifi,
        timer,
        esp_hal::rng::Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut led = io.pins.gpio12.into_push_pull_output();

    let wifi = peripherals.WIFI;
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (iface, device, mut controller, sockets) =
        create_network_interface(&init, wifi, WifiStaDevice, &mut socket_set_entries).unwrap();
    let wifi_stack = WifiStack::new(iface, device, sockets, current_millis);
    info!("配置WIFI ssid和password");
    let client_config = Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASS.try_into().unwrap(),
        ..Default::default()
    });
    info!("配置WIFI");
    controller.set_configuration(&client_config).ok();
    info!("启动WIFI");
    controller.start().ok();
    info!("启动WIFI成功：{:?}", controller.is_started());
    info!("连接WIFI");
    controller.connect().unwrap();
    loop {
        let res = controller.is_connected();
        match res {
            Ok(connected) => {
                if connected {
                    break;
                }
            }
            Err(err) => {
                error!("WIFI连接失败：{:?}", err);
                delay.delay_millis(3000);
            }
        }
    }
    info!("WIFI连接成功：{:?}", controller.is_connected());

    info!("等待获取IP地址");
    loop {
        wifi_stack.work();
        if wifi_stack.is_iface_up() {
            info!("获取到的IP地址：{:?}", wifi_stack.get_ip_info());
            break;
        }
    }

    let mut storage = esp_storage::FlashStorage::new();
    let mut ota = ota::Ota::new(&mut storage);
    let current_slot = ota.current_slot();
    info!("current slot: {:?}", current_slot);
    let new_slot = current_slot.next();

    let mut rx_buffer = [0u8; 1536];
    let mut tx_buffer = [0u8; 1536];
    let mut socket = wifi_stack.get_socket(&mut rx_buffer, &mut tx_buffer);

    // 获取ota升级文件
    socket.work();
    socket
        .open(IpAddress::Ipv4(Ipv4Address::new(192, 168, 31, 129)), 80)
        .unwrap();
    socket
        .write(b"GET / HTTP/1.0\r\nHost: 192.168.31.129\r\n\r\n")
        .unwrap();
    socket.flush().unwrap();

    let wait_end = current_millis() + 20 * 1000;
    loop {
        let mut buffer = [0u8; 512];
        if let Ok(len) = socket.read(&mut buffer) {
            let to_print = unsafe { core::str::from_utf8_unchecked(&buffer[..len]) };
            info!("{}", to_print);
        } else {
            break;
        }

        if current_millis() > wait_end {
            info!("Timeout");
            break;
        }
    }

    socket.disconnect();

    let wait_end = current_millis() + 5 * 1000;
    while current_millis() < wait_end {
        socket.work();
    }

    loop {
        led.toggle();
        delay.delay_millis(500);
    }
}

fn parse_ip(ip: &str) -> Ipv4Address {
    let mut result = [0u8; 4];
    for (idx, octet) in ip.split(".").into_iter().enumerate() {
        result[idx] = u8::from_str_radix(octet, 10).unwrap();
    }

    Ipv4Address::from_bytes(&result)
}
