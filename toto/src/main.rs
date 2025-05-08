use core::convert::TryInto;
use core::error;

use embedded_svc::http::client::Client;
use embedded_svc::http::Method;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use embedded_svc::{http::client::Client as HttpClient, io::Write, utils::io};
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::http::client::Configuration as HttpClientConfiguration;
use esp_idf_svc::http::client::EspHttpConnection;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::tls::{Config, EspTls};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use log::info;
use serde::Deserialize;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const SSID: &'static str = "Mezei";
const PASSWORD: &'static str = "Hs-fhU%3~MC";

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    unsafe {
        esp_idf_sys::esp_tls_init_global_ca_store();
    }

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    use std::sync::Mutex;

    let led = PinDriver::output(peripherals.pins.gpio2)?;
    let led = Arc::new(Mutex::new(led));
    let led_clone = Arc::clone(&led);

    thread::spawn(move || loop {
        let mut led = led_clone.lock().unwrap();
        led.set_high().unwrap();
        thread::sleep(Duration::from_millis(500));
        led.set_low().unwrap();
        thread::sleep(Duration::from_millis(500));
    });

    connect_wifi(&mut wifi)?;

    #[derive(Deserialize)]
    struct TimeApiResponse {
        datetime: String,
    }

    let current_time = fetch_current_time()?;
    info!("Current time from API: {current_time}");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {ip_info:?}");

    info!("Shutting down in 5s...");

    std::thread::sleep(core::time::Duration::from_secs(5));

    Ok(())
}

fn fetch_current_time() -> anyhow::Result<String> {
    info!("Fetching current time from API...");

    let mut client = Client::wrap(EspHttpConnection::new(&HttpClientConfiguration {
        use_global_ca_store: true, // ha az esp-idf build beállítás engedélyezi
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?);

    let request = client.get("https://ipapi.co/8.8.8.8/json/")?;

    let mut response = request.submit()?;

    info!("Response status: {}", response.status());
    let mut buf = [0u8; 1024];
    let bytes_read = io::try_read_full(&mut response, &mut buf).map_err(|e| e.0)?;
    info!("Read {bytes_read} bytes");
    match std::str::from_utf8(&buf[0..bytes_read]) {
        Ok(body_string) => return Ok(body_string.to_string()),
        Err(e) => {
            return Err(anyhow::anyhow!("Error decoding response body: {e}"));
        }
    };
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(())
}
