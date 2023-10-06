use crate::models::action::{Action, Command, PrinterAction, PrinterStatus, TelemetryData};
use crate::models::file::{FinishedPrint, GcodeFile};
use crate::models::serial_connector::SerialConnector;
use crate::serial::event_loop::Serial;
use event_listener::Event;
use lazy_static::lazy_static;
use log::{debug, warn};
use regex::Regex;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast::{Receiver, Sender};

lazy_static! {
    static ref RE_MAX_Z_POS: Regex = Regex::new(r";\s*max_layer_z\s*=\s*([\d.]+)").unwrap();
}

pub struct GcodeSerial {
    tx: Sender<Action>,
    que: Arc<Mutex<VecDeque<String>>>,
    event: Arc<Mutex<Event>>,
    active_file: Option<GcodeFile>,
}

impl GcodeSerial {
    pub fn new(tx: Sender<Action>) -> Self {
        let q = Arc::new(Mutex::new(VecDeque::new()));
        let event = Arc::new(Mutex::new(Event::new()));
        GcodeSerial {
            tx: tx.clone(),
            que: q,
            event,
            active_file: None,
        }
    }

    /// connect to printer and initialize lib
    pub async fn start(&mut self, serial_connector: SerialConnector) {
        let rx = self.tx.subscribe();

        let que = self.que.clone();
        let event = self.event.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut serial = Serial::new(tx, serial_connector, que, event).await;
            serial.start_temp_interval();
            serial.start_event_loop().await;
        });

        self.handle_action_commands(rx).await;
    }

    /// set target temperatures of bed and extruder
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

    /// start a new print of given gcode file path
    /// won't start file if event que size > 10
    pub fn start_print(&mut self, file_path: String) {
        // if we have a large que we don't do anything
        let que_len = self.que.lock().unwrap().len();
        if que_len > 10 {
            warn!(
                "Failed to start new print. Que has still {} elements",
                que_len
            );
            return;
        }

        let file = File::open(&file_path).unwrap();

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

        let active_file = GcodeFile {
            name: file_path,
            size,
            last_modified: unix_timestamp,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        };
        self.active_file = Some(active_file.clone());

        let _ = self
            .tx
            .send(Action::Telemetry(TelemetryData::ActiveFileChange(Some(
                active_file,
            ))));

        self.load_file_to_que(reader);

        let _ = self
            .tx
            .send(Action::Telemetry(TelemetryData::TotalCommandCount(
                self.que.lock().unwrap().len() as u32,
            )));
        let _ = self.tx.send(Action::StateChange(PrinterStatus::Active));
        self.event.lock().unwrap().notify(42);
    }

    /// stop the active print and add the stop gcode to que
    pub fn stop_print(&self) {
        self.clear_que_add_ending_code();

        let _ = self
            .tx
            .send(Action::Telemetry(TelemetryData::TotalCommandCount(0)));
        let _ = self.tx.send(Action::StateChange(PrinterStatus::Idle));
        self.event.lock().unwrap().notify(42);
    }

    async fn handle_action_commands(&mut self, mut rx: Receiver<Action>) {
        while let Ok(v) = rx.recv().await {
            match v {
                Action::Telemetry(_) => {}
                Action::StateChange(s) => {
                    match s {
                        PrinterStatus::Disconnected => {}
                        PrinterStatus::Active => {}
                        PrinterStatus::Idle => {
                            if self.active_file.is_some() {
                                let f = self.active_file.clone().unwrap();

                                let _ = self.tx.send(Action::Telemetry(
                                    TelemetryData::PrintFinished(FinishedPrint {
                                        name: f.name,
                                        size: f.size,
                                        last_modified: f.last_modified,
                                        start_time: f.start_time,
                                        finish_time: SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_millis(),
                                    }),
                                ));
                                self.active_file = None;
                            }
                        }
                        PrinterStatus::Errored => {}
                    }
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

    fn load_file_to_que(&self, reader: BufReader<File>) {
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

            // if line starts with ; or is empty we skip it
            if command.trim().starts_with(';') || command.trim().is_empty() {
                continue;
            }

            // we remove comments and take the gcode cmd only
            if command.trim().contains(';') {
                command = command.trim().split(';').collect::<Vec<&str>>()[0].to_string();
            }

            self.que.lock().unwrap().push_back(command);
        }
    }

    fn clear_que_add_ending_code(&self) {
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
    }
}
