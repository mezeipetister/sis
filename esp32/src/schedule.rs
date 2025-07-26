use crate::{BoardEvent, Program, Schedule};
use chrono::{Datelike, Local, NaiveDateTime, Utc};
use crossbeam::channel::{self, Receiver, Sender};
use crossbeam::select;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
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
    next_program_opt: Option<Program>,
    wait_duration: Duration,
    nvs: EspNvs<NvsDefault>,
}

impl ScheduleModule {
    pub fn new(
        tx: Sender<BoardEvent>,
        esp_partition: EspNvsPartition<NvsDefault>,
    ) -> (Self, Sender<ScheduleCommand>) {
        let (cmd_tx, rx) = channel::unbounded();

        let next_program_opt = None;
        let wait_duration = Duration::from_secs(0);

        let nvs = EspNvs::new(esp_partition, "storage", true).expect("Failed to create NVS");

        let mut res = Self {
            rx,
            tx,
            schedule: None,
            next_program_opt,
            wait_duration,
            nvs,
        };

        res.load_schedule_from_nvs()
            .expect("Failed to load schedule from NVS");

        if let Some(schedule) = &res.schedule {
            let _ = res.tx.send(BoardEvent::ScheduleLoaded {
                version: schedule.version,
            });
        } else {
            info!("No schedule found in NVS.");
        }

        // Set the initial the next program
        res.set_next_program();

        (res, cmd_tx)
    }

    fn set_next_program(&mut self) {
        // Set the initial the next program
        let (next_program_opt, wait_duration) = match Self::calculate_next_program(&self.schedule) {
            Some((prog, dur)) => (Some(prog), dur),
            None => (None, Duration::from_secs(600)), // Default wait if no program is scheduled
        };
        self.next_program_opt = next_program_opt;
        self.wait_duration = wait_duration;
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("schedule_module".into())
            .stack_size(8192)
            .spawn(move || {
                self.run();
            })
            .expect("Failed to spawn schedule thread");
    }

    fn run(mut self) {
        loop {
            self.set_next_program();

            let next_prog_opt = self.next_program_opt.clone();
            let wait_duration = self.wait_duration;

            let timer_rx = channel::after(wait_duration);

            let tick_rx = channel::tick(Duration::from_secs(1));

            println!(
                "Next program id: {:?} with wait secs {}",
                next_prog_opt.as_ref().map(|p| &p.id),
                wait_duration.as_secs()
            );

            select! {
                recv(self.rx) -> msg => {
                    match msg {
                        Ok(ScheduleCommand::UpdateSchedule(new_sched)) => {
                            // let version = new_sched.version;
                            self.schedule = Some(new_sched.clone());
                            info!("Schedule updated to version {}", &new_sched.version);

                            // Save the schedule to NVS
                            if let Err(e) = self.save_schedule_to_nvs(&new_sched) {
                                info!("Failed to save schedule to NVS: {}", e);
                            }

                            // Recalculate the next program
                            self.set_next_program();

                            let _ = self.tx.send(BoardEvent::ScheduleUpdated { version: self.schedule.clone().unwrap_or_default().version });
                        }

                        Ok(ScheduleCommand::StartProgramById(id)) => {
                            if let Some(schedule) = &self.schedule {
                                if let Some(prog) = schedule
                                    .programs
                                    .iter()
                                    .find(|p| p.id == id)
                                {
                                    let _ = self.tx.send(BoardEvent::ProgramStarted { program: prog.clone() });
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

                recv(timer_rx) -> _ => {
                    if let Some(prog) = next_prog_opt.as_ref() {
                        let _ = self.tx.send(BoardEvent::ProgramStarted { program: prog.clone() });
                        self.set_next_program();
                        info!("Program started automatically.");
                    }
                }

                recv(tick_rx) -> _ => {
                    // csak hogy életben tartsuk a szálat
                    // ide tehetsz időzített státuszfrissítést is, ha kell
                    continue;
                }
            }
        }
    }

    fn calculate_next_program(schedule: &Option<Schedule>) -> Option<(Program, Duration)> {
        let now = Local::now();
        let now_date = now.date_naive();
        let now_time = now.time();

        let mut best: Option<(Program, NaiveDateTime)> = None;

        if let Some(schedule) = schedule {
            'outer: for add_days in 0..7 {
                let date = now_date + chrono::Duration::days(add_days);
                let weekday = date.weekday().number_from_monday() as i8;

                for prog in &schedule.programs {
                    if !prog.active || !prog.weekdays.contains(&weekday) {
                        continue;
                    }

                    let start_time = prog.start_time;

                    // Ha ma vagyunk, de a start_time már elmúlt, kihagyjuk
                    if add_days == 0 && start_time <= now_time {
                        continue;
                    }

                    let dt = date.and_time(start_time);
                    if let Some(start_dt) = dt.and_local_timezone(Utc).latest() {
                        if best.is_none() || start_dt.naive_utc() < best.as_ref().unwrap().1 {
                            best = Some((prog.clone(), start_dt.naive_utc()));
                        }
                    }

                    if best.is_some() && add_days == 0 {
                        // ha ma találunk egy jót, kiléphetünk korán
                        break 'outer;
                    }
                }
            }
        }

        best.map(|(prog, start_dt)| {
            let dur = start_dt
                .signed_duration_since(now.naive_local())
                .to_std()
                .unwrap_or(Duration::from_secs(0));
            (prog, dur)
        })
    }

    fn save_schedule_to_nvs(&mut self, schedule: &Schedule) -> anyhow::Result<()> {
        let data = bincode::serialize(&schedule)?;
        self.nvs.set_raw("schedule_bin", &data)?;
        info!("Schedule saved to NVS. Version: {}", schedule.version);

        Ok(())
    }

    fn load_schedule_from_nvs(&mut self) -> anyhow::Result<()> {
        let mut buf = vec![0u8; 12288];
        let _ = self.nvs.get_raw("schedule_bin", &mut buf)?;
        if buf.len() > 0 {
            let schedule: Schedule = bincode::deserialize(&buf)?;
            info!("Schedule loaded from NVS. Version: {}", schedule.version);
            self.schedule = Some(schedule);
        } else {
            info!("No schedule found in NVS.");
        }
        Ok(())
    }
}
