use std::{
    thread,
    time::{Duration, Instant},
};

use crossbeam::{
    channel::{Receiver, Sender},
    select,
};
use esp_idf_svc::hal::gpio::{AnyIOPin, Output, Pin, PinDriver};
use log::info;

use crate::{BoardEvent, Program, ZoneAction};

pub trait RelayPin: Send {
    fn set_high(&mut self);
    fn set_low(&mut self);
}

impl<P: Pin + Into<AnyIOPin>> RelayPin for PinDriver<'static, P, Output> {
    fn set_high(&mut self) {
        self.set_high().unwrap();
    }

    fn set_low(&mut self) {
        self.set_low().unwrap();
    }
}

pub struct Relay {
    id: String,
    pin: Box<dyn RelayPin>,
}

impl Relay {
    pub fn new<T: RelayPin + 'static>(id: String, pin: T) -> Self {
        Relay {
            id,
            pin: Box::new(pin),
        }
    }

    pub fn open(&mut self) {
        self.pin.set_high();
    }

    pub fn close(&mut self) {
        self.pin.set_low();
    }
}

pub struct RelayController {
    relays: Vec<Relay>,
}

impl RelayController {
    pub fn new(relays: Vec<Relay>) -> Self {
        Self { relays }
    }

    fn close_all(&mut self) {
        for relay in &mut self.relays {
            relay.close();
        }
        info!("All relays closed");
    }

    fn open(&mut self, ids: Vec<String>) {
        self.close_all();
        for relay in &mut self.relays {
            if ids.contains(&relay.id) {
                relay.open();
            }
        }
        info!("Relays opened: {:?}", ids);
    }

    pub fn get_zones(&self) -> Vec<String> {
        self.relays.iter().map(|r| r.id.clone()).collect()
    }
}

pub enum RelayCommand {
    StartProgram(Program),
    StartZoneAction(ZoneAction),
    Stop,
}

pub struct RelayModule {
    relay_controller: RelayController,
    tx: Sender<BoardEvent>,
    rx: Receiver<RelayCommand>,
}

impl RelayModule {
    pub fn start(self) {
        thread::spawn(move || {
            self.run();
        });
    }

    pub fn new(
        relay_controller: RelayController,
        tx: Sender<BoardEvent>,
    ) -> (Self, Sender<RelayCommand>) {
        let (module_tx, rx) = crossbeam::channel::unbounded::<RelayCommand>();
        (
            Self {
                relay_controller,
                tx,
                rx,
            },
            module_tx,
        )
    }

    pub fn run(mut self) {
        let mut current_zone_index: Option<usize> = None;
        let mut current_program: Option<Program> = None;
        let mut zone_start_time: Option<Instant> = None;

        loop {
            select! {
                recv(self.rx) -> msg => {
                    match msg {
                        Ok(RelayCommand::Stop) => {
                            self.relay_controller.close_all();
                            self.tx.send(BoardEvent::ProgramStopped).ok();
                            self.tx.send(BoardEvent::ZoneActionStopped).ok();
                            current_zone_index = None;
                            current_program = None;
                            zone_start_time = None;
                        },
                        Ok(RelayCommand::StartZoneAction(zone)) => {
                            self.tx.send(BoardEvent::ZoneActionStarted { zone_action: zone.clone() }).ok();
                            self.relay_controller.open(zone.zone_ids.clone());
                            zone_start_time = Some(Instant::now());
                            current_zone_index = Some(0);
                            current_program = Some(Program {
                                id: "single".into(),
                                name: "Ad-hoc".into(),
                                weekdays: vec![],
                                start_time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                                active: true,
                                zones: vec![zone],
                            });
                        },
                        Ok(RelayCommand::StartProgram(prog)) => {
                            self.tx.send(BoardEvent::ProgramRunning { program: prog.clone() }).ok();

                            // Open the first zone of the program
                            if let Some(first_zone) = prog.zones.get(0) {
                                self.tx.send(BoardEvent::ZoneActionStarted { zone_action: first_zone.clone() }).ok();
                                self.relay_controller.open(first_zone.zone_ids.clone());
                            }

                            current_program = Some(prog);
                            current_zone_index = Some(0);
                            zone_start_time = Some(Instant::now());

                        },
                        Err(_) => break, // channel closed
                    }
                },
                default(Duration::from_millis(200)) => {
                    if let (Some(ref prog), Some(index), Some(start)) =
                        (&current_program, current_zone_index, zone_start_time)
                    {
                        let elapsed = Instant::now().duration_since(start).as_secs();

                        if let Some(zone) = prog.zones.get(index) {
                            if elapsed >= zone.duration_seconds as u64 {
                                self.relay_controller.close_all();
                                self.tx.send(BoardEvent::ZoneActionStopped).ok();

                                let next_index = index + 1;
                                if let Some(next_zone) = prog.zones.get(next_index) {
                                    self.tx.send(BoardEvent::ZoneActionStarted {
                                        zone_action: next_zone.clone(),
                                    }).ok();
                                    self.relay_controller.open(next_zone.zone_ids.clone());
                                    current_zone_index = Some(next_index);
                                    zone_start_time = Some(Instant::now());
                                } else {
                                    self.tx.send(BoardEvent::ProgramStopped).ok();
                                    current_program = None;
                                    current_zone_index = None;
                                    zone_start_time = None;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
