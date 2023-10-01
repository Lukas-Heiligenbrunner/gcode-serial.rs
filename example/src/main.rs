use gcode_serial::gcode_serial::GcodeSerial;
use gcode_serial::models::action::{Action, Command, PrinterStatus, TelemetryData};

use tokio::sync::broadcast;
use tokio::runtime::Runtime;
use gcode_serial::models::serial_connector::SerialConnector;

fn main() {
    // initialize Tokio runtime
    let t = Runtime::new().unwrap();

    let filename = "test.gcode";

    t.block_on(async move {
        let (tx, _) = broadcast::channel(32);

        let t = tx.clone();
        tokio::spawn(async move {
            // create printer object
            let mut pa = GcodeSerial::new(t);
            // start printer service
            pa.start(SerialConnector::Auto).await;
        });

        // send print start command
        let _ = tx.send(Action::Command(Command::StartPrint(filename.to_string())));

        let mut rx = tx.subscribe();
        // monitor for receiving actions
        loop {
            if let Ok(v) = rx.recv().await {
                match v {
                    Action::Telemetry(t) => {
                        match t {
                            TelemetryData::Temps(v) => {
                                println!("Extruder temp: {}; Bed temp: {}", v.ex_temp, v.bed_temp);
                            }
                            TelemetryData::Progress(x) => {
                                println!("Number of commands left in que: {}", x);
                            }
                            TelemetryData::TargetExtruderTemp(t) => {
                                println!("Target Extruder Temp change: {}", t);
                            }
                            TelemetryData::TargetBedTemp(t) => {
                                println!("Target Extruder Temp change: {}", t);
                            }
                            TelemetryData::TotalCommandCount(n) => {
                                println!("Number of total commands to send: {}", n);
                            }
                            TelemetryData::ActiveFileChange(f) => {
                                println!("Current file changed: {}", f.unwrap().name);
                            }
                            _ => {}
                        }
                    }
                    Action::StateChange(s) => {
                        match s {
                            PrinterStatus::Disconnected => {}
                            PrinterStatus::Active => {}
                            PrinterStatus::Idle => {
                                println!("print finished");
                                break;
                            }
                            PrinterStatus::Errored => {
                                println!("print errored");
                                break;
                            }
                        }
                    }
                    Action::PrinterAction(_) => {}
                    Action::Command(_) => {}
                }
            }
        }
    })
}
