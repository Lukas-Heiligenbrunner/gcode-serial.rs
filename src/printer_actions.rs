use crate::action::{Action, Command, PrinterAction, PrinterStatus, TelemetryData};
use crate::file::{FilamentType, GcodeFile};
use crate::serial::Serial;
use event_listener::Event;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
use tokio::sync::broadcast::Sender;

lazy_static! {
    static ref RE_MAX_Z_POS: Regex = Regex::new(r";\s*max_layer_z\s*=\s*([\d.]+)").unwrap();
}

pub struct PrinterActions {
    tx: Sender<Action>,
    que: Arc<Mutex<VecDeque<String>>>,
    event: Arc<Mutex<Event>>,
}

impl PrinterActions {
    pub fn new(tx: Sender<Action>) -> Self {
        let q = Arc::new(Mutex::new(VecDeque::new()));
        let event = Arc::new(Mutex::new(Event::new()));
        PrinterActions {
            tx: tx.clone(),
            que: q,
            event,
        }
    }

    pub async fn start(&mut self) {
        let mut rx = self.tx.subscribe();

        let que = self.que.clone();
        let event = self.event.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut serial = Serial::new(tx, 115_200, que, event).await;
            serial.start_temp_interval();
            serial.start_event_loop().await;
        });

        while let Ok(v) = rx.recv().await {
            match v {
                Action::Telemetry(_) => {}
                Action::StateChange(s) => {
                    debug!("Printer State change: {}", s);
                }
                Action::PrinterAction(a) => match a {
                    PrinterAction::Cancel => {
                        self.stop_print();
                    }
                    PrinterAction::Pause => {
                        todo!()
                    }
                    PrinterAction::Resume => {
                        todo!()
                    }
                },
                Action::Command(c) => match c {
                    Command::SetTemps(b, c) => {
                        self.set_temps(b, c);
                    }
                    Command::StartPrint(n) => {
                        self.start_print(n);
                    }
                    Command::StopPrint => {
                        self.stop_print();
                    }
                },
            }
        }
    }

    pub fn set_temps(&mut self, bed_temp: u16, extruder_temp: u16) {
        self.que
            .lock()
            .unwrap()
            .push_back(format!("M140 S{}", bed_temp));
        self.que
            .lock()
            .unwrap()
            .push_back(format!("M104 S{}", extruder_temp));
    }

    pub fn start_print(&self, file_name: String) {
        // if !self.data.lock().unwrap().printer_connection_alive {
        //     return Err("The printer might not be started or connected.".to_string());
        // }

        // if self.data.lock().unwrap().active_file.is_some() {
        //     return Err("already started?".to_string());
        // }

        // if we have a large que we don't do anything
        if self.que.lock().unwrap().len() > 10 {
            return;
        }

        let file = File::open(format!("models/{}", file_name)).unwrap();

        let unix_timestamp = file
            .metadata()
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let size = file.metadata().unwrap().len();

        let reader = BufReader::new(file);

        // self.data.lock().unwrap().print_job_active = true;
        // todo
        let active_file = Some(GcodeFile {
            name: file_name,
            size,
            last_modified: unix_timestamp,
            filament_type: FilamentType::PLA, // todo get correct type here
        });
        let _ = self
            .tx
            .send(Action::Telemetry(TelemetryData::ActiveFile(active_file)));

        for line in reader.lines() {
            let mut command = line.unwrap();

            // match for max_layer_z commet to get max layyer height - might be prusaslicer only!
            match RE_MAX_Z_POS.captures(command.as_str()) {
                None => {}
                Some(c) => {
                    let h1: f32 = c
                        .get(1)
                        .map_or("0.0", |m| m.as_str())
                        .parse()
                        .unwrap_or(0.0);
                    let _ = self
                        .tx
                        .send(Action::Telemetry(TelemetryData::MaxZHeight(h1)));
                }
            }

            if command.trim().starts_with(";") || command.trim().is_empty() {
                continue;
            }

            if command.trim().contains(";") {
                command = command.trim().split(";").collect::<Vec<&str>>()[0].to_string();
            }

            self.que.lock().unwrap().push_back(command);
        }

        let _ = self
            .tx
            .send(Action::Telemetry(TelemetryData::TotalCommandCount(
                self.que.lock().unwrap().len() as u32,
            )));
        let _ = self.tx.send(Action::StateChange(PrinterStatus::Active));
        self.event.lock().unwrap().notify(42);
    }

    pub fn stop_print(&self) {
        let mut que = self.que.lock().unwrap();
        que.clear();
        // run end procedure gcodes
        que.push_back("G1 X0 Y200 F3600".to_string()); // park
        que.push_back("G4".to_string()); // wait
        que.push_back("M221 S100".to_string()); // reset flow
        que.push_back("M900 K0".to_string()); // reset LA
        que.push_back("M104 S0".to_string()); // turn off temperature
        que.push_back("M140 S0".to_string()); // turn off heatbed
        que.push_back("M107".to_string()); // turn off fan
        que.push_back("M84".to_string()); // disable motors
                                          // que.push_back("M603".to_string()); // prusa specific gcode-endprint
        let _ = self
            .tx
            .send(Action::Telemetry(TelemetryData::TotalCommandCount(0)));
        let _ = self.tx.send(Action::StateChange(PrinterStatus::Idle));
        self.event.lock().unwrap().notify(42);
    }
}
