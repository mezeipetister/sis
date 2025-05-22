use chrono::{TimeZone, Utc};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use serde::Serialize;

use crate::{get_mac, relay::RelayController, BoardEvent, ZoneAction};

#[derive(Serialize, Default, Clone)]
pub struct BoardInfo {
    device_id: String,
    datetime: String,
    schedule_version: i32,
    running_program: Option<String>,
    running_zones: Option<ZoneAction>,
    zones: Vec<String>,
    log: Option<String>,
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
            log: None,
        }
    }

    // Apply board event to update the BoardInfo
    // Returns Some(updated BoardInfo) if the event was applied, None otherwise
    pub fn apply_event(&mut self, event: &BoardEvent) -> Option<Self> {
        match event {
            BoardEvent::DateTimeUpdated { time } => {
                self.datetime = Utc.from_utc_datetime(time).to_rfc3339();
                self.log = Some(format!(
                    "DateTime updated to {}",
                    Utc.from_utc_datetime(time).to_rfc3339()
                ));
                Some(self.clone())
            }
            BoardEvent::WsStatusChanged { connected: _ } => None,
            BoardEvent::WifiStatusChanged { status: _ } => None,
            BoardEvent::ServerCommandArrived { command: _ } => None,
            // Board stored new schedule
            // Update schedule version
            BoardEvent::ScheduleUpdated { version } => {
                if self.schedule_version != *version {
                    self.schedule_version = *version;
                    self.log = Some(format!("Schedule updated to version {}", version));
                    Some(self.clone()) // új állapot, küldeni kell
                } else {
                    None // nincs változás, ne küldjük újra
                }
            }
            // Board has just started
            // Update schedule version based on the nvr stored schedule
            BoardEvent::ScheduleLoaded { version } => {
                if self.schedule_version != *version {
                    self.schedule_version = *version;
                    self.log = Some(format!("Schedule loaded from NVR to version {}", version));
                    Some(self.clone()) // új állapot, küldeni kell
                } else {
                    None // nincs változás, ne küldjük újra
                }
            }
            BoardEvent::ProgramStarted { program: _ } => None,
            // Board started a program
            // Update running program
            BoardEvent::ProgramRunning { program } => {
                self.running_program = Some(program.id.clone());
                self.log = Some(format!("Program started: {}", program.name));
                Some(self.clone())
            }
            // Board stopped a program
            // Update running program
            BoardEvent::ProgramStopped => {
                self.running_program = None;
                self.log = Some("Program stopped".to_string());
                Some(self.clone())
            }
            // Board started a zone action
            // Update running zones
            BoardEvent::ZoneActionStarted { zone_action } => {
                self.running_zones = Some(zone_action.clone());
                self.log = Some(format!(
                    "Zone action started: {}",
                    zone_action
                        .zone_ids
                        .iter()
                        .map(|z| z.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                ));
                Some(self.clone())
            }
            // Board stopped a zone action
            // Update running zones
            BoardEvent::ZoneActionStopped => {
                self.running_zones = None;
                self.log = Some("Zone action stopped".to_string());
                Some(self.clone())
            }
        }
    }
}
