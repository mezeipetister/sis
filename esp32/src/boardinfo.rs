use chrono::{TimeZone, Utc};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use serde::Serialize;

use crate::{get_mac, BoardEvent, RelayController, ZoneAction};

#[derive(Serialize, Default, Clone)]
pub struct BoardInfo {
    device_id: String,
    datetime: String,
    schedule_version: i32,
    running_program: Option<String>,
    running_zones: Option<ZoneAction>,
    zones: Vec<String>,
}

impl BoardInfo {
    pub fn init(
        wifi: &BlockingWifi<EspWifi<'static>>,
        relay_controller: &RelayController,
        schedule_version: i32,
    ) -> Self {
        // Get the MAC address of the device
        let device_id = get_mac(wifi).unwrap();
        // Create a new BoardInfo instance
        let datetime = Utc::now().to_rfc3339();
        // Init zones from relay controller
        let zones = relay_controller.get_zones();

        Self {
            device_id,
            datetime,
            schedule_version,
            running_program: None,
            running_zones: None,
            zones,
        }
    }

    // Apply board event to update the BoardInfo
    // Returns Some(updated BoardInfo) if the event was applied, None otherwise
    pub fn apply_event(&mut self, event: &BoardEvent) -> Option<Self> {
        match event {
            BoardEvent::DateTimeUpdated { time } => {
                self.datetime = Utc.from_utc_datetime(time).to_rfc3339();
                Some(self.clone())
            }
            BoardEvent::WsStatusChanged { connected: _ } => None,
            BoardEvent::WifiStatusChanged { status: _ } => None,
            BoardEvent::ServerCommandArrived { command: _ } => None,
        }
    }
}
