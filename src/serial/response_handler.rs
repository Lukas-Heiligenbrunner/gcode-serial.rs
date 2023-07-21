use crate::models::action::{Action, PrinterAction, PrinterStatus, TelemetryData};
use crate::models::temperature::Temperature;
use crate::serial::serial::Serial;
use lazy_static::lazy_static;
use log::warn;
use regex::Regex;
use std::time::{SystemTime, UNIX_EPOCH};

lazy_static! {
    static ref RE_M105: Regex = Regex::new(r".*T:([\d.]+)\s\/([\d.]+)\sB:([\d.]+)\s\/([\d.]+)\s.*").unwrap();
    // regex while heating process:
    static ref RE_HEATING: Regex = Regex::new(r".*T:([\d.]+)\s.*B:([\d.]+).*").unwrap();

    // NORMAL MODE: Percent done: 68; print time remaining in mins: 8; Change in mins: -1
    static ref RE_SD_PRINT: Regex = Regex::new(r"NORMAL MODE: Percent done: ([\d]+); print time remaining in mins: ([\d]+); .*").unwrap();

    // regex to match action commands
    static ref RE_ACTION_COMMAND: Regex = Regex::new(r".*\/\/\s+action:(\w*).*").unwrap();
}

impl Serial {
    pub(crate) fn handle_response(&self, line: &str) {
        // capture a M105 temp response
        match RE_M105.captures(line) {
            None => {}
            Some(c) => {
                let h1: f32 = c.get(1).map_or("0", |m| m.as_str()).parse().unwrap_or(0.0);
                let h1_t: f32 = c.get(2).map_or("0", |m| m.as_str()).parse().unwrap_or(0.0);

                let b1: f32 = c.get(3).map_or("0", |m| m.as_str()).parse().unwrap_or(0.0);
                let b1_t: f32 = c.get(4).map_or("0", |m| m.as_str()).parse().unwrap_or(0.0);

                let temp = Temperature {
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    bed_temp: b1,
                    ex_temp: h1,
                };

                self.send_telemetry(TelemetryData::Temps(temp.clone()));
                self.send_telemetry(TelemetryData::TargetBedTemp(b1_t as u32));
                self.send_telemetry(TelemetryData::TargetExtruderTemp(h1_t as u32));
            }
        }

        // capture a while heating temp response
        match RE_HEATING.captures(line) {
            None => {}
            Some(c) => {
                let h1: f32 = c.get(1).map_or("0", |m| m.as_str()).parse().unwrap_or(0.0);
                let b1: f32 = c.get(2).map_or("0", |m| m.as_str()).parse().unwrap_or(0.0);

                let temp = Temperature {
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    bed_temp: b1,
                    ex_temp: h1,
                };

                self.send_telemetry(TelemetryData::Temps(temp.clone()))
            }
        }

        // this regex applies only to sdcard-prints
        match RE_SD_PRINT.captures(line) {
            None => {}
            Some(c) => {
                let c1 = c.get(1).map_or("0", |m| m.as_str());
                let c2 = c.get(2).map_or("0", |m| m.as_str());

                let percent_done: u32 = c1.parse().unwrap_or(0);
                self.send_telemetry(TelemetryData::PercentDone(percent_done));
                let mins_remaining: u32 = c2.parse().unwrap_or(0);
                self.send_telemetry(TelemetryData::MinsRemaining(mins_remaining));
            }
        }

        // handle action responses
        match RE_ACTION_COMMAND.captures(line) {
            None => {}
            Some(c) => {
                let action = c.get(1).map_or("0", |m| m.as_str());
                match action {
                    "cancel" => {
                        //let _ = self.p_actions.stop_print();
                        let _ = self.tx.send(Action::PrinterAction(PrinterAction::Cancel));
                        warn!("canceling print job");
                    }
                    "pause" => {
                        // todo send correct pause command
                        // wait for continue from printer
                        let _ = self.tx.send(Action::PrinterAction(PrinterAction::Pause));
                        warn!("Pause Command from Printer received");
                    }
                    "resume" => {
                        // resume print - but how to get this message?
                        let _ = self.tx.send(Action::PrinterAction(PrinterAction::Resume));
                        warn!("Continue Command from Printer received");
                    }
                    v => {
                        warn!("Unknown action Command received: {}", v);
                    }
                }
            }
        }

        if line.contains("Done printing file") {
            self.update_status(PrinterStatus::Idle);
        }
    }
}
