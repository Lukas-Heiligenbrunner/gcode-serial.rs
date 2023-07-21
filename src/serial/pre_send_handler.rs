use crate::models::action::TelemetryData;
use crate::serial::serial::Serial;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // regex to match G1 height changes - expected to have no start spaces
    static ref RE_Z_CHANGE: Regex = Regex::new(r"G1\s.*Z([\d.]+).*").unwrap();

    //  regex to match target temp change
    static ref RE_TARGET_TEMP: Regex = Regex::new(r".*M1\d\d\sS(\d+).*").unwrap();

    // regex to match a fan speed change gcode
    static ref RE_FAN_SPEED: Regex = Regex::new(r".*M106\s.*S([\d.]+)*").unwrap();
}

impl Serial {
    pub(crate) fn handle_presend_cmd(&self, cmd: &str) {
        // if M104/M140 commands are sent, read target temperatures
        if cmd.contains("M104 S") {
            match RE_TARGET_TEMP.captures(cmd) {
                None => {}
                Some(c) => {
                    let h1_t: u32 = c.get(1).map_or("0", |m| m.as_str()).parse().unwrap_or(0);
                    if h1_t != 0 {
                        self.send_telemetry(TelemetryData::TargetExtruderTemp(h1_t));
                    }
                }
            }
        } else if cmd.contains("M140 S") {
            match RE_TARGET_TEMP.captures(cmd) {
                None => {}
                Some(c) => {
                    let h1_t: u32 = c.get(1).map_or("0", |m| m.as_str()).parse().unwrap_or(0);
                    if h1_t != 0 {
                        self.send_telemetry(TelemetryData::TargetBedTemp(h1_t));
                    }
                }
            }
        }

        // capture a Z height change
        match RE_Z_CHANGE.captures(cmd) {
            None => {}
            Some(c) => {
                let h1: f32 = c
                    .get(1)
                    .map_or("-1", |m| m.as_str())
                    .parse()
                    .unwrap_or(-1.0);

                if h1 >= 0.0 {
                    self.send_telemetry(TelemetryData::ZHeight(h1));
                }
            }
        }

        // capture fan speed change
        match RE_FAN_SPEED.captures(cmd) {
            None => {}
            Some(c) => {
                let h1: f32 = c
                    .get(1)
                    .map_or("-1", |m| m.as_str())
                    .parse()
                    .unwrap_or(-1.0);

                if h1 >= 0.0 {
                    self.send_telemetry(TelemetryData::FanSpeed(h1 / 255.0));
                }
            }
        }
    }
}
