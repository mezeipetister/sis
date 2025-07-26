use crate::BoardEvent;
use chrono::Utc;
use crossbeam::channel::Sender;
use ds3231::{
    Config, InterruptControl, Oscillator, SquareWaveFrequency, TimeRepresentation, DS3231,
};
use esp_idf_svc::{
    hal::i2c::I2cDriver,
    sntp::{self, EspSntp, SyncStatus},
};
use log::info;
use std::{thread, time::Duration};

pub struct TimeController {
    rtc: DS3231<I2cDriver<'static>>,
    sntp: EspSntp<'static>,
    tx: Sender<BoardEvent>,
    inited: bool,
}

impl TimeController {
    // Create a new TimeController instance
    // This function initializes the DS3231 RTC and sets the time.
    pub fn new(tx: Sender<BoardEvent>, i2c: I2cDriver<'static>) -> Self {
        let config = Config {
            time_representation: TimeRepresentation::TwentyFourHour,
            square_wave_frequency: SquareWaveFrequency::Hz1,
            interrupt_control: InterruptControl::SquareWave,
            battery_backed_square_wave: false,
            oscillator_enable: Oscillator::Enabled,
        };

        info!("Initializing DS3231...");

        let mut rtc = DS3231::new(i2c, 0x68);

        info!("DS3231 initialized");

        let _sntp = sntp::EspSntp::new_default().expect("Failed to initialize SNTP");
        info!("SNTP initialized");

        // Configure the device
        rtc.configure(&config).expect("Failed to configure DS3231");
        let mut controller = TimeController {
            rtc,
            sntp: _sntp,
            tx,
            inited: false,
        };
        controller.init_dtime_from_ds3231();
        controller
    }

    // Set the time on the DS3231 RTC
    // This function reads the current date and time from the DS3231 RTC
    // and sets it to the system time.
    fn init_dtime_from_ds3231(&mut self) {
        if let Ok(datetime) = get_dtime_from_ds3231(&mut self.rtc) {
            let _ = self.tx.send(BoardEvent::DateTimeUpdated { time: datetime });
        }
    }

    pub fn start_sntp_sync(self) {
        thread::Builder::new()
            .name("schedule_module".into())
            .stack_size(8192) // vagy próbáld: 8192 vagy 16384
            .spawn(move || {
                self.run();
            })
            .expect("Failed to spawn schedule thread");
    }

    fn run(mut self) {
        loop {
            match self.sntp.get_sync_status() {
                SyncStatus::Completed => {
                    info!("SNTP synchronized");
                    let current_board_utc_time = Utc::now().naive_utc();
                    info!("Current UTC time on board: {current_board_utc_time}");
                    set_dtime_to_ds3231(&mut self.rtc, current_board_utc_time)
                        .expect("Failed to set time to DS3231");
                    let _ = self.tx.send(BoardEvent::DateTimeUpdated {
                        time: current_board_utc_time,
                    });
                    break;
                }
                SyncStatus::InProgress => {
                    info!("SNTP not synchronized");
                }
                SyncStatus::Reset => {
                    info!("SNTP reset");
                }
            }
            thread::sleep(Duration::from_secs(20));
        }

        let now = Utc::now().naive_utc();
        info!("Current UTC time from systime: {now}");
    }
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

pub enum TimeCommand {}
