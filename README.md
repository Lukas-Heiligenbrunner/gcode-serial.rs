# A Rust lib to send Gcode via UART to a 3D printer
A stream based GCode sender.
The api is still in progress and subject to change.

## Quick start Code:


```rust
use gcode_serial::gcode_serial::GcodeSerial;
use gcode_serial::models::action::{Action, Command, PrinterStatus, TelemetryData};
use gcode_serial::models::serial_connector::SerialConnector;

use tokio::sync::broadcast;
use tokio::runtime::Runtime;

fn main() {
    // initialize Tokio runtime
    let t = Runtime::new().unwrap();

    let filename = "test.gcode";

    t.block_on(async move {
        let (tx, _) = broadcast::channel(32);

        let t = tx.clone();
        tokio::spawn(async move {
            // create printer object
            let mut gs = GcodeSerial::new(t);
            // start printer service
            gs.start(SerialConnector::Auto).await;
        });

        // send print start command
        tx.send(Action::Command(Command::StartPrint(filename)));

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
                            TelemetryData::ActiveFile(f) => {
                                println!("Current file changed: {}", f.unwrap().name);
                            }
                            _ => {}
                        }
                    }
                    Action::StateChange(_) => {}
                    Action::PrinterAction(_) => {}
                    Action::Command(_) => {}
                }
            }
        }
    })
}

```

## Usage

### GcodeSerial object
At first you need to create an instance of `GcodeSerial` for each printer you want to connect with
`let mut gs = GcodeSerial::new(t)`
where t is a broadcast channel needed to cummunicate with the lib. 

Afterwards you can connect to the printer with `gs.start(SerialConnector::Auto).await;`.
You might wanna call this in a seperate (soft)-thread because this call is blocking.

### SerialConnector
You can choose if the serial port should be found automatically with `SerialConnector::Auto` or manually with 
`SerialConnector::Manual("/dev/ttyUSB0", 115_200)`

### Receiving/Sending data from/to the printer
Attach to the broadcast stream and receive with the Enum the data you need. See example above. 

| Enum Field               | Function                | Example                                        | Trigger      |
|--------------------------|-------------------------|------------------------------------------------|--------------|
| Action::Telemetry(t)     | receive telemetry       | temperature, progress, target temps, fan speed | value change |
| Action::StateChange(t)   | printer state change    | Disconnected,Active,Idle,Errored,              | value change |
| Action::PrinterAction(t) | printer sent action cmd | cancel/pause/resume print                      | printer      |
| Action::Command(t)       | send actions to the lib | start/cancel print, Set Temperatures           | lib user     |

## License

MIT License

Copyright (c) 2023 Lukas Heiligenbrunner

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
