#![no_std]
#![no_main]

use embedded_io::{Read, Write};
use esp_backtrace as _;
use esp_hal::{clock::ClockControl, delay::Delay, peripherals::Peripherals, prelude::*};
use esp_wifi::{
    current_millis,
    wifi::{ClientConfiguration, Configuration, WifiStaDevice},
    wifi_interface::WifiStack,
};
use log::{error, info};
use smoltcp::{
    iface::SocketStorage,
    wire::{IpAddress, Ipv4Address},
};

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

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let delay = Delay::new(&clocks);
    init_heap();

    esp_println::logger::init_logger_from_env();

    let timer = esp_hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    info!("初始化WIFI");
    let wifi_init = esp_wifi::initialize(
        esp_wifi::EspWifiInitFor::Wifi,
        timer,
        esp_hal::rng::Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (iface, device, mut controller, sockets) = esp_wifi::wifi::utils::create_network_interface(
        &wifi_init,
        wifi,
        WifiStaDevice,
        &mut socket_set_entries,
    )
    .unwrap();
    let wifi_stack = WifiStack::new(iface, device, sockets, esp_wifi::current_millis);

    info!("配置WIFI_SSID和WIFI_PASS");
    let client_conf = Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASS.try_into().unwrap(),
        ..Default::default()
    });
    let res = controller.set_configuration(&client_conf);
    info!("WIFI配置结果：{res:?}");

    info!("开始启动WIFI");
    controller.start().ok();
    info!("WIFI是否启动成功：{:?}", controller.is_started());

    info!("开始WIFI扫描");
    let res = controller.scan_n::<10>();
    if let Ok((res, _count)) = res {
        log::info!("扫描到的AP");
        for ap in res {
            info!("{:?}", ap.ssid);
        }
    }

    info!("WIFI能力：{:?}", controller.get_capabilities());

    let delay = Delay::new(&clocks);

    info!("开始连接WIFI");
    controller.connect().ok();
    loop {
        match controller.is_connected() {
            Ok(connected) => {
                if connected {
                    break;
                }
            }
            Err(e) => {
                error!("WIFI连接失败：{e:?}");
                delay.delay_millis(5000);
            }
        }
    }
    info!("WIFI是否连接成功： {:?}", controller.is_connected());

    info!("等待获取IP地址");
    loop {
        wifi_stack.work();

        if wifi_stack.is_iface_up() {
            info!("获取到的IP地址：{:?}", wifi_stack.get_ip_info());
            break;
        }
    }

    info!("Start busy loop on main");

    let mut rx_buffer = [0u8; 1536];
    let mut tx_buffer = [0u8; 1536];
    let mut socket = wifi_stack.get_socket(&mut rx_buffer, &mut tx_buffer);

    loop {
        info!("Making HTTP request");
        socket.work();

        socket
            .open(IpAddress::Ipv4(Ipv4Address::new(142, 250, 185, 115)), 80)
            .unwrap();

        socket
            .write(b"GET / HTTP/1.0\r\nHost: www.mobile-j.de\r\n\r\n")
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
    }
}
