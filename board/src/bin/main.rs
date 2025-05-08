#![no_std]
#![no_main]

use core::fmt::Write;
use core::net::Ipv4Addr;
use esp_hal::i2c::master::I2c;
use esp_hal::peripheral::Peripheral;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::{Config, Uart};
use esp_hal::{
    clock::CpuClock,
    main,
    time::{Duration, Instant},
};
use esp_hal::{time, Blocking};
use esp_println::println;
use esp_wifi::wifi::WifiMode;
use esp_wifi::{
    init,
    wifi::{ClientConfiguration, Configuration},
};
use log::info;
use smoltcp::{
    iface::{SocketSet, SocketStorage},
    wire::{DhcpOption, IpAddress},
};

use blocking_network_stack::Stack;
use ds323x::Rtcc;
use ds323x::{Ds323x, NaiveDateTime};

const SSID: &str = "mezei";
const PASSWORD: &str = "Hs-fhU%3~MC";

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// #[main]
// fn main() -> ! {
//     esp_idf_sys::link_patches();
//     esp_idf_svc::log::EspLogger::initialize_default();

//     let peripherals = Peripherals::take().unwrap();
//     let i2c = I2cDriver::new(
//         peripherals.i2c0,
//         peripherals.pins.gpio21, // SDA
//         peripherals.pins.gpio22, // SCL
//         &I2cConfig::default(),
//     )?;

//     let mut rtc = Ds323x::new_ds3231(i2c);

//     // 1. Lekérjük az időt az RTC modulból
//     match rtc.get_datetime() {
//         Ok(datetime) => info!("RTC idő (kezdeti): {:?}", datetime),
//         Err(e) => warn!("Nem sikerült RTC időt lekérni: {:?}", e),
//     }

//     // 2. Lekérjük az időt REST API-ból
//     let client = Client::wrap(EspHttpConnection::new_default()?);
//     let request = client.get("http://worldtimeapi.org/api/timezone/Europe/Budapest")?;
//     let response = request.submit()?;

//     #[derive(Debug, Deserialize)]
//     struct TimeResponse {
//         datetime: String,
//     }

//     let time_data: TimeResponse = serde_json::from_reader(response)?;
//     info!("Idő REST API-ból: {}", time_data.datetime);

//     // 3. Konvertáljuk a szöveget chrono::NaiveDateTime formátumra
//     let dt_str = time_data.datetime;
//     let naive = chrono::NaiveDateTime::parse_from_str(&dt_str[..19], "%Y-%m-%dT%H:%M:%S")?;
//     info!("Pontos idő: {:?}", naive);

//     // 4. Beállítjuk az RTC modult erre az időre
//     rtc.set_datetime(&naive)?;
//     info!("RTC idő beállítva");

//     // 5. Újra lekérjük és logoljuk
//     let updated = rtc.get_datetime()?;
//     info!("RTC idő frissítés után: {:?}", updated);

//     loop {
//         sleep(Duration::from_secs(60));
//     }
// }

#[main]
fn main() -> ! {
    // generator version: 0.3.1
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timg0.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let (input, output) = peripherals.GPIO2.split();

    let mut uart0 = Uart::new(peripherals.UART0, Config::default()).unwrap();

    info!("Hello, world!");

    output.enable_output(true);

    let mut rng = Rng::new(peripherals.RNG);

    let esp_wifi_ctrl = init(timg0.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap();

    let (mut controller, interfaces) =
        esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();

    let mut device = interfaces.sta;
    let iface = create_interface(&mut device);

    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let mut socket_set = SocketSet::new(&mut socket_set_entries[..]);
    let mut dhcp_socket = smoltcp::socket::dhcpv4::Socket::new();
    // we can set a hostname here (or add other DHCP options)
    dhcp_socket.set_outgoing_options(&[DhcpOption {
        kind: 12,
        data: b"esp-wifi",
    }]);
    socket_set.add(dhcp_socket);

    let now = || time::Instant::now().duration_since_epoch().as_millis();
    let stack = Stack::new(iface, device, socket_set, now, rng.random());

    controller
        .set_power_saving(esp_wifi::config::PowerSaveMode::None)
        .unwrap();

    let client_config = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASSWORD.into(),
        ..Default::default()
    });
    let res = controller.set_configuration(&client_config);
    println!("wifi_set_configuration returned {:?}", res);

    controller.start().unwrap();
    println!("is wifi started: {:?}", controller.is_started());

    println!("Start Wifi Scan");
    let res = controller.scan_n(10).unwrap();
    for ap in res {
        println!("{:?}", ap);
    }

    println!("{:?}", controller.capabilities());
    println!("wifi_connect {:?}", controller.connect());

    // wait to get connected
    println!("Wait to get connected");
    loop {
        match controller.is_connected() {
            Ok(true) => break,
            Ok(false) => {}
            Err(err) => {
                println!("{:?}", err);
                loop {}
            }
        }
    }
    println!("{:?}", controller.is_connected());

    // wait for getting an ip address
    println!("Wait to get an ip address");
    loop {
        stack.work();

        if stack.is_iface_up() {
            println!("got ip {:?}", stack.get_ip_info());
            break;
        }
    }

    println!("Start busy loop on main");

    let mut rx_buffer = [0u8; 1536];
    let mut tx_buffer = [0u8; 1536];
    let mut socket = stack.get_socket(&mut rx_buffer, &mut tx_buffer);

    loop {
        println!("Making HTTP request");
        socket.work();

        socket
            .open(IpAddress::Ipv4(Ipv4Addr::new(142, 250, 185, 115)), 80)
            .unwrap();

        socket
            .write(b"GET / HTTP/1.0\r\nHost: www.mobile-j.de\r\n\r\n")
            .unwrap();
        socket.flush().unwrap();

        let deadline = time::Instant::now() + Duration::from_secs(20);
        let mut buffer = [0u8; 512];
        while let Ok(len) = socket.read(&mut buffer) {
            let to_print = unsafe { core::str::from_utf8_unchecked(&buffer[..len]) };
            print!("{}", to_print);

            if time::Instant::now() > deadline {
                println!("Timeout");
                break;
            }
        }
        println!();

        socket.disconnect();

        let deadline = time::Instant::now() + Duration::from_secs(5);
        while time::Instant::now() < deadline {
            socket.work();
        }
    }

    let mut i2c = I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default())
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22);

    let mut rtc = Ds323x::new_ds3231(i2c);

    match rtc.time() {
        Ok(datetime) => info!("RTC time: {:?}", datetime),
        Err(e) => info!("Failed to read RTC time: {:?}", e),
    }

    loop {
        let delay_start = Instant::now();
        output.set_output_high(true);
        info!("LED ON");
        while delay_start.elapsed() < Duration::from_millis(1000) {}

        let delay_start = Instant::now();
        output.set_output_high(false);
        info!("LED OFF");
        match rtc.time() {
            Ok(datetime) => info!("RTC time: {:?}", datetime),
            Err(e) => info!("Failed to read RTC time: {:?}", e),
        }
        while delay_start.elapsed() < Duration::from_millis(1000) {}
    }
}

fn timestamp() -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_micros(
        esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_micros() as i64,
    )
}
pub fn create_interface(device: &mut esp_wifi::wifi::WifiDevice) -> smoltcp::iface::Interface {
    // users could create multiple instances but since they only have one WifiDevice
    // they probably can't do anything bad with that
    smoltcp::iface::Interface::new(
        smoltcp::iface::Config::new(smoltcp::wire::HardwareAddress::Ethernet(
            smoltcp::wire::EthernetAddress::from_bytes(&device.mac_address()),
        )),
        device,
        timestamp(),
    )
}
