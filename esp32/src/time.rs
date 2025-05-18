use chrono::NaiveDateTime;
use ds3231::DS3231;
use esp_idf_svc::hal::i2c::I2cDriver;

struct TimeController<'a> {
    rtc: DS3231<I2cDriver<'a>>,
}

impl<'a> TimeController<'a> {
    // Create a new TimeController instance
    // This function initializes the DS3231 RTC and sets the time.
    pub fn new(rtc: DS3231<I2cDriver<'a>>) -> Self {
        let mut controller = TimeController { rtc };
        controller.set_time();
        controller
    }

    // Set the time on the DS3231 RTC
    // This function reads the current date and time from the DS3231 RTC
    // and sets it to the system time.
    fn set_time(&mut self) {
        let datetime = get_dtime_from_ds3231(&mut self.rtc).unwrap();
        set_system_time_from_naive(datetime).unwrap();
    }
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
