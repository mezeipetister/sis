use crate::{BoardEvent, Program, Schedule};
use chrono::{Datelike, Local, NaiveDateTime, Utc};
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
    next_program_opt: Option<Program>,
    wait_duration: Duration,
}

impl ScheduleModule {
    pub fn new(
        schedule: Option<Schedule>,
        tx: Sender<BoardEvent>,
    ) -> (Self, Sender<ScheduleCommand>) {
        let (cmd_tx, rx) = channel::unbounded();

        let next_program_opt = None;
        let wait_duration = Duration::from_secs(0);

        let mut res = Self {
            rx,
            tx,
            schedule,
            next_program_opt,
            wait_duration,
        };

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
            .stack_size(4096) // vagy próbáld: 8192 vagy 16384
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

                            // Recalculate the next program
                            self.set_next_program();

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

                recv(timer_rx) -> _ => {
                    if let Some(prog) = next_prog_opt.as_ref() {
                        self.tx.send(BoardEvent::ProgramStarted { program: prog.clone() }).ok();
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
}
