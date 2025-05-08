#![no_std]
#![no_main]

use core::fmt::Write;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::{Config, Uart};
use esp_hal::Blocking;
use esp_hal::{
    clock::CpuClock,
    main,
    time::{Duration, Instant},
};
use log::info;

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

fn log(urt: &mut Uart<'_, Blocking>, msg: &str) {
    writeln!(urt, "{}", msg).unwrap();
}

#[main]
fn main() -> ! {
    // generator version: 0.3.1
    // esp_idf_svc::log::EspLogger::initialize_default();

    // Logger inicializálása UART-ra

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

    let mut led = peripherals.GPIO2;

    let mut uart0 = Uart::new(peripherals.UART0, Config::default()).unwrap();

    log(&mut uart0, "Hello, world!");

    // loop {
    //     led.split().1.set_output_high(true);
    //     timg0.timer0.start(Duration::from_secs(1)).unwrap();
    //     while timg0.timer0.is_running().unwrap() {}
    //     led.set_low().unwrap();
    //     timg0.timer0.delay(Duration::from_secs(1));
    // }
    let (input, output) = led.split();

    output.enable_output(true);

    loop {
        let delay_start = Instant::now();
        output.set_output_high(true);
        log(&mut uart0, "LED ON");
        while delay_start.elapsed() < Duration::from_millis(1000) {}

        let delay_start = Instant::now();
        output.set_output_high(false);
        log(&mut uart0, "LED OFF");
        while delay_start.elapsed() < Duration::from_millis(1000) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}
