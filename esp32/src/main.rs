use boardinfo::BoardInfo;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use core::convert::TryInto;
use core::error;
use ds3231::{
    Config as DsConfig, InterruptControl, Ocillator, SquareWaveFrequency, TimeRepresentation,
    DS3231,
};
use embedded_svc::http::client::Client;
use embedded_svc::http::Method;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use embedded_svc::{http::client::Client as HttpClient, io::Write, utils::io};
use esp_idf_svc::hal::gpio::{AnyIOPin, Output, Pin, PinDriver};
use esp_idf_svc::hal::i2c::config::Config as I2cConfig;
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::hal::peripherals;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::http::client::Configuration as HttpClientConfiguration;
use esp_idf_svc::http::client::EspHttpConnection;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspNvs;
use esp_idf_svc::sntp::{self, SyncStatus};
use esp_idf_svc::systime::EspSystemTime;
use esp_idf_svc::tls::{Config, EspTls};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_sys::tzset;
use log::info;
use relay::{Relay, RelayController};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

// const RELAY_PIN_MAP: [(i32, i32); 8] = [
//     (1, 2),
//     (2, 4),
//     (3, 5),
//     (4, 25),
//     (5, 26),
//     (6, 18),
//     (7, 19),
//     (8, 23), // kamu
// ];

mod boardinfo;
mod relay;
mod schedule;
mod time;
mod ws;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZoneAction {
    zone_ids: Vec<String>,
    duration_seconds: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Program {
    id: String,
    name: String,
    weekdays: Vec<i8>,
    start_time: NaiveTime,
    active: bool,
    zones: Vec<ZoneAction>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Schedule {
    version: i32,
    programs: Vec<Program>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ServerCommand {
    SetNewSchedule(Schedule),
    Stop,
    StartZoneAction(ZoneAction),
    StartProgram(String),
}

#[derive(Debug, Clone)]
pub enum BoardEvent {
    ScheduleUpdated { version: i32 },
    ScheduleLoaded { version: i32 },
    ProgramStarted { program: Program },
    ProgramRunning { program: Program },
    ProgramStopped,
    ZoneActionStarted { zone_action: ZoneAction },
    ZoneActionStopped,
    DateTimeUpdated { time: NaiveDateTime },
    WsStatusChanged { connected: bool },
    WifiStatusChanged { status: bool },
    ServerCommandArrived { command: ServerCommand },
}

// Set system time from NaiveDateTime
// This function sets the system time using the provided NaiveDateTime.
// It converts the NaiveDateTime to a timestamp (in seconds)
// and creates a TimeVal struct to pass to the settimeofday function.
// The function returns Ok(()) on success and Err(()) on failure.
// The settimeofday function is an external C function that sets the system time.
// It takes a pointer to a TimeVal struct and a pointer to a timezone struct (not used here).
#[repr(C)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

extern "C" {
    fn settimeofday(tv: *const TimeVal, tz: *const core::ffi::c_void) -> i32;
}

// Set system time from NaiveDateTime
// This function sets the system time using the provided NaiveDateTime.
// It converts the NaiveDateTime to a timestamp (in seconds)
// and creates a TimeVal struct to pass to the settimeofday function.
// The function returns Ok(()) on success and Err(()) on failure.
// The settimeofday function is an external C function that sets the system time.
// It takes a pointer to a TimeVal struct and a pointer to a timezone struct (not used here).
// The timezone struct is passed as null since we are not using it.
fn set_system_time_from_naive(dt: NaiveDateTime) -> Result<(), ()> {
    let timestamp = dt.and_utc().timestamp(); // i64 (UTC másodperc)
    let tv = TimeVal {
        tv_sec: timestamp,
        tv_usec: 0,
    };
    let result = unsafe { settimeofday(&tv, std::ptr::null()) };
    if result == 0 {
        Ok(())
    } else {
        Err(())
    }
}

// Get MAC address as a string
// This function is used to get the MAC address of the device
// and format it as a string in the format "xx:xx:xx:xx:xx:xx"
// where x is a hexadecimal digit.
// It uses the EspWifi API to get the MAC address from the network interface.
// The function takes a reference to a BlockingWifi instance
// and returns a Result containing the MAC address string or an error.
fn get_mac(wifi: &BlockingWifi<EspWifi<'static>>) -> anyhow::Result<String> {
    let mac = wifi.wifi().sta_netif().get_mac()?;
    let mac_str = format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    Ok(mac_str)
}

// Connect to WiFi
// This function is used to connect to a WiFi network.
// It takes a mutable reference to a BlockingWifi instance
// and returns a Result indicating success or failure.
// The function creates a Configuration object with the SSID and password,
// sets the configuration on the wifi instance, starts the wifi,
// and waits for the network interface to be up.
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

// Read time from DS3231
// This function reads the current date and time from the DS3231 RTC.
fn get_dtime_from_ds3231(rtc: &mut DS3231<I2cDriver>) -> anyhow::Result<chrono::NaiveDateTime> {
    // Get current date/time
    let datetime = rtc.datetime().unwrap();
    Ok(datetime.into())
}

// Set date/time to DS3231
// This function sets the date and time on the DS3231 RTC.
fn set_dtime_to_ds3231(
    rtc: &mut DS3231<I2cDriver>,
    datetime: chrono::NaiveDateTime,
) -> anyhow::Result<()> {
    rtc.set_datetime(&datetime).unwrap();
    Ok(())
}

// fn bcd_to_decimal(bcd: u8) -> u8 {
//     ((bcd >> 4) * 10) + (bcd & 0x0F)
// }

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    unsafe {
        esp_idf_sys::esp_tls_init_global_ca_store();
    }

    let (s, r) = crossbeam::channel::unbounded::<BoardEvent>();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;

    let default = EspDefaultNvsPartition::take().unwrap();

    let default_clone = default.clone();

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(default_clone))?,
        sys_loop,
    )?;

    use std::fs::File;
    use std::io::{self, Write as IoWrite};
    use std::sync::mpsc;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::Mutex;

    // let led = PinDriver::output(peripherals.pins.gpio2)?;
    // let led = Arc::new(Mutex::new(led));
    // let led_clone = Arc::clone(&led);

    let mac_address_str = get_mac(&wifi)?;
    info!("MAC Address: {}", mac_address_str);

    #[derive(Deserialize)]
    struct TimeApiResponse {
        datetime: String,
    }

    // Initialize device with I2C
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &I2cConfig {
            ..Default::default()
        },
    )?;

    let config = DsConfig {
        time_representation: TimeRepresentation::TwentyFourHour,
        square_wave_frequency: SquareWaveFrequency::Hz1,
        interrupt_control: InterruptControl::SquareWave,
        battery_backed_square_wave: false,
        oscillator_enable: Ocillator::Enabled,
    };

    info!("Initializing DS3231...");

    let mut rtc = DS3231::new(i2c, 0x68);

    info!("DS3231 initialized");

    // Configure the device
    rtc.configure(&config).expect("Failed to configure DS3231");

    let current_utc_time = get_dtime_from_ds3231(&mut rtc)?;
    info!("Current local time from DS3231: {current_utc_time}");

    // Set the system time from the DS3231
    set_system_time_from_naive(current_utc_time).unwrap();

    let now = Utc::now().naive_utc();
    info!("Current UTC time from systime: {now}");

    let utc_logger_handle = thread::spawn(move || loop {
        let current_utc_time = Utc::now().naive_utc();
        info!("Current UTC time: {}", current_utc_time);
        thread::sleep(Duration::from_secs(1));
    });

    let mac = get_mac(&wifi)?;

    let relay_pins: Vec<Relay> = vec![
        Relay::new(
            format!("{mac}/1"),
            PinDriver::output(peripherals.pins.gpio2)?,
        ),
        Relay::new(
            format!("{mac}/2"),
            PinDriver::output(peripherals.pins.gpio4)?,
        ),
        Relay::new(
            format!("{mac}/3"),
            PinDriver::output(peripherals.pins.gpio5)?,
        ),
        Relay::new(
            format!("{mac}/4"),
            PinDriver::output(peripherals.pins.gpio25)?,
        ),
        Relay::new(
            format!("{mac}/5"),
            PinDriver::output(peripherals.pins.gpio26)?,
        ),
        Relay::new(
            format!("{mac}/6"),
            PinDriver::output(peripherals.pins.gpio18)?,
        ),
        Relay::new(
            format!("{mac}/7"),
            PinDriver::output(peripherals.pins.gpio19)?,
        ),
    ];

    // RelayController initialization
    let mut relay_controller = RelayController::new(relay_pins);

    // BoardInfo initialization
    let mut boardinfo = BoardInfo::init(&wifi, &relay_controller, 0);

    // thread::spawn(move || loop {
    //     for i in 1..=7 {
    //         relay_controller.open(vec![format!("{mac}/{i}")]);
    //         thread::sleep(Duration::from_secs(1));
    //     }
    //     relay_controller.open(
    //         (1..=relay_controller.relays.len())
    //             .map(|i| format!("{mac}/{i}"))
    //             .collect(),
    //     );
    //     thread::sleep(Duration::from_secs(1));
    // });

    connect_wifi(&mut wifi)?;

    let mac = get_mac(&wifi)?;
    info!("MAC Address: {}", mac);

    let _sntp = sntp::EspSntp::new_default()?;
    info!("SNTP initialized");

    loop {
        match _sntp.get_sync_status() {
            SyncStatus::Completed => {
                info!("SNTP synchronized");
                let current_board_utc_time = Utc::now().naive_utc();
                info!("Current UTC time on board: {current_board_utc_time}");
                set_dtime_to_ds3231(&mut rtc, current_board_utc_time)?;
                break;
            }
            SyncStatus::InProgress => {
                info!("SNTP not synchronized");
            }
            SyncStatus::Reset => {
                info!("SNTP reset");
            }
        }
        thread::sleep(Duration::from_secs(1));
    }

    let now = Utc::now().naive_utc();
    info!("Current UTC time from systime: {now}");

    let (tx, rx) = crossbeam::channel::unbounded::<BoardEvent>();

    // Init WsModule
    let (ws_module, ws_tx) = ws::WsModule::new(
        format!("ws://192.168.88.30:3400/websocket"),
        "hellobello".to_string(),
        tx.clone(),
    );

    // Start WebSocket module
    ws_module.start();
    info!("WebSocket client started");

    // Init relay module
    let (relay_module, relay_tx) = relay::RelayModule::new(relay_controller, tx.clone());
    // Start relay module
    relay_module.start();

    let default_clone = default.clone();

    // Init schedule module
    let (schedule_module, schedule_tx) = schedule::ScheduleModule::new(tx.clone(), default_clone);

    // Start schedule module
    schedule_module.start();

    loop {
        match rx.recv() {
            Ok(event) => {
                // Try to apply the event to the boardinfo
                // and send the updated boardinfo to the WebSocket
                if let Some(updated_boardinfo) = boardinfo.apply_event(&event) {
                    ws_tx
                        .send(ws::WsCommand::NewBoardInfo(updated_boardinfo))
                        .unwrap();
                }

                // info!("Received BoardEvent: {:?}", event);

                match event {
                    BoardEvent::DateTimeUpdated { time } => {
                        info!("DateTime updated: {}", time);
                        set_system_time_from_naive(time).ok();
                        set_dtime_to_ds3231(&mut rtc, time).ok();
                    }
                    BoardEvent::WsStatusChanged { connected } => {
                        info!("WebSocket status changed: connected={}", connected);
                        // Send connect command if not connected to WS process
                        if !connected {
                            ws_tx.send(ws::WsCommand::Connect).unwrap();
                        } else {
                            ws_tx
                                .send(ws::WsCommand::NewBoardInfo(boardinfo.clone()))
                                .unwrap();
                        }
                    }
                    BoardEvent::WifiStatusChanged { status } => {
                        info!("WiFi status changed: status={}", status);
                    }
                    BoardEvent::ServerCommandArrived { command } => {
                        info!("Server command arrived: {:?}", command);
                        match command {
                            ServerCommand::SetNewSchedule(schedule) => {
                                info!("New schedule received: version={}", schedule.version);
                                schedule_tx
                                    .send(schedule::ScheduleCommand::UpdateSchedule(schedule))
                                    .unwrap();
                            }
                            ServerCommand::Stop => {
                                info!("Stop command received");
                                relay_tx.send(relay::RelayCommand::Stop).unwrap();
                            }
                            ServerCommand::StartZoneAction(zone_action) => {
                                info!("StartZoneAction command received: {:?}", zone_action);
                                relay_tx
                                    .send(relay::RelayCommand::StartZoneAction(zone_action.clone()))
                                    .unwrap();
                            }
                            ServerCommand::StartProgram(program_id) => {
                                info!("StartProgram command received: {}", program_id);
                                schedule_tx
                                    .send(schedule::ScheduleCommand::StartProgramById(program_id))
                                    .unwrap();
                            }
                        }
                    }
                    BoardEvent::ScheduleUpdated { version } => (),
                    BoardEvent::ScheduleLoaded { version } => (),
                    BoardEvent::ProgramStarted { program } => {
                        info!("Program started: {}", program.name);
                        relay_tx
                            .send(relay::RelayCommand::StartProgram(program.clone()))
                            .unwrap();
                    }
                    BoardEvent::ProgramRunning { program } => (),
                    BoardEvent::ProgramStopped => (),
                    BoardEvent::ZoneActionStarted { zone_action } => (),
                    BoardEvent::ZoneActionStopped => (),
                }
            }
            Err(e) => {
                info!("Error receiving BoardEvent: {:?}", e);
                break;
            }
        }
    }

    // let current_time = demo_api_call()?;
    // info!("Current time from API: {current_time}");

    // let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    // info!("Wifi DHCP info: {ip_info:?}");

    // info!("Shutting down in 5s...");

    Ok(())
}

// fn demo_api_call() -> anyhow::Result<String> {
//     info!("Fetching current time from API...");

//     let mut client = Client::wrap(EspHttpConnection::new(&HttpClientConfiguration {
//         use_global_ca_store: true, // ha az esp-idf build beállítás engedélyezi
//         crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
//         ..Default::default()
//     })?);

//     let request = client.get("https://ipapi.co/8.8.8.8/json/")?;

//     let mut response = request.submit()?;

//     info!("Response status: {}", response.status());
//     let mut buf = [0u8; 1024];
//     let bytes_read = io::try_read_full(&mut response, &mut buf).map_err(|e| e.0)?;
//     info!("Read {bytes_read} bytes");
//     match std::str::from_utf8(&buf[0..bytes_read]) {
//         Ok(body_string) => return Ok(body_string.to_string()),
//         Err(e) => {
//             return Err(anyhow::anyhow!("Error decoding response body: {e}"));
//         }
//     };
// }
