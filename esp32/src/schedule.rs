use crate::{BoardEvent, Program, Schedule};
use chrono::{Datelike, Local, Utc};
use crossbeam::channel::{self, Receiver, Sender};
use crossbeam::select;
use log::info;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum ScheduleCommand {
    UpdateSchedule(Schedule),
    StartProgramById(String),
}

pub struct ScheduleModule {
    rx: Receiver<ScheduleCommand>,
    tx: Sender<BoardEvent>,
    schedule: Option<Schedule>,
}

impl ScheduleModule {
    pub fn new(
        schedule: Option<Schedule>,
        tx: Sender<BoardEvent>,
    ) -> (Self, Sender<ScheduleCommand>) {
        let (cmd_tx, rx) = channel::unbounded();

        (Self { rx, tx, schedule }, cmd_tx)
    }

    pub fn start(mut self) {
        thread::spawn(move || {
            // let mut timer_rx = None;
            loop {
                // let (next_prog_opt, wait_duration) =
                //     ScheduleModule::calculate_next_program(&self.schedule);

                // let (next_prog_opt, wait_duration) = Self::calculate_next_program(&self.schedule);
                // let timer_rx = channel::after(wait_duration);

                select! {
                    recv(self.rx) -> msg => {
                        match msg {
                            Ok(ScheduleCommand::UpdateSchedule(new_sched)) => {
                              // let version = new_sched.version;
                              self.schedule = Some(new_sched.clone());
                              info!("Schedule updated to version {}", &new_sched.version);
                              self.tx.send(BoardEvent::ScheduleUpdated { version: self.schedule.clone().unwrap_or_default().version }).ok();
                            }

                            Ok(ScheduleCommand::StartProgramById(id)) => {
                                if let Some(schedule) = &self.schedule {
                                    if let Some(prog) = schedule
                                        .programs
                                        .iter()
                                        .find(|p| p.id == id)
                                    {
                                        self.tx.send(BoardEvent::ProgramStarted { program: prog.clone() }).ok();
                                        info!("Program started by ID: {}", id);
                                    } else {
                                        info!("Program with ID {} not found", id);
                                    }
                                } else {
                                    info!("No schedule available to start program");
                                }
                            }

                            Err(_) => {
                                info!("ScheduleModule command channel closed.");
                                break;
                            }
                        }
                    }

                    // recv(timer_rx) -> _ => {
                    //   if let Some(prog) = next_prog_opt {
                    //         self.tx.send(BoardEvent::ProgramStarted { program: prog }).ok();
                    //         info!("Program started automatically.");
                    //     }
                    // }
                }
            }
        });
    }

    // fn calculate_next_program(schedule: &Option<Schedule>) -> (Option<Program>, Duration) {
    //     let now = Local::now();
    //     let weekday = now.weekday().number_from_monday() as i8;
    //     let now_time = now.time();

    //     if let Some(schedule) = schedule {
    //         let mut programs = schedule.programs.clone();
    //         programs.sort_by_key(|p| p.start_time);

    //         for prog in programs {
    //             if !prog.active || !prog.weekdays.contains(&weekday) {
    //                 continue;
    //             }

    //             if prog.start_time > now_time {
    //                 let start_dt = now.date_naive().and_time(prog.start_time);
    //                 if let Some(start_dt) = start_dt.and_local_timezone(Utc).single() {
    //                     let dur = match start_dt.signed_duration_since(now).to_std() {
    //                         Ok(d) => d,
    //                         Err(_) => continue,
    //                     };
    //                     return (Some(prog), dur);
    //                 }
    //             }
    //         }
    //     }

    //     (None, Duration::from_secs(600))
    // }
}
