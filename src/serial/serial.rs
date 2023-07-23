use crate::models::action::{Action, PrinterStatus, TelemetryData};
use crate::models::serial_connector::SerialConnector;

use event_listener::Event;
use log::{debug, error, info, warn};
use serialport::{ClearBuffer, SerialPort};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::Instant;
use tokio::sync::broadcast::Sender;

pub struct Serial {
    port: Box<dyn SerialPort>,
    pub(crate) que: Arc<Mutex<VecDeque<String>>>,
    pub(crate) event: Arc<Mutex<Event>>,
    pub(crate) tx: Sender<Action>,
    pub(crate) printer_status: Arc<Mutex<PrinterStatus>>,
}

impl Serial {
    pub async fn new(
        tx: Sender<Action>,
        serial_connector: SerialConnector,
        qq: Arc<Mutex<VecDeque<String>>>,
        event: Arc<Mutex<Event>>,
    ) -> Self {
        let (name, boud) = match serial_connector {
            SerialConnector::Auto => {
                let ports = loop {
                    let ports = serialport::available_ports().unwrap_or(Vec::new());
                    debug!("Number of ports: {}", ports.len());
                    for p in &ports {
                        debug!("PORT: {}", p.port_name);
                    }

                    if ports.len() != 0 {
                        break ports;
                    } else {
                        warn!("No Serial port found, retrying in 5secs!");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                };

                let pname = &ports[0].port_name;
                let name = pname.split("/").last().unwrap();

                (format!("/dev/{}", name), 115_200)
            }
            SerialConnector::Manual(serial_port, boud) => (serial_port, boud),
        };

        let mut p = serialport::new(name, boud)
            .timeout(Duration::from_millis(10000))
            .open()
            .expect("cannot open port");

        p.write("\r\n\r\n".as_bytes())
            .expect("failed to write init");
        p.flush().expect("failed to flush");

        tokio::time::sleep(Duration::from_millis(2000)).await;

        let nrtoread = p.bytes_to_read().expect("unable to get read buffer nr");
        let nrtowrite = p.bytes_to_write().expect("unable to get write buffer nr");
        debug!("nr of bytes to read: {}", nrtoread);
        debug!("nr of bytes to write: {}", nrtowrite);

        p.clear(ClearBuffer::All)
            .expect("failed to clear input buffer");

        p.set_timeout(Duration::from_millis(100))
            .expect("failed to set printer timeout");

        Serial {
            port: p,
            que: qq,
            event,
            tx,
            printer_status: Arc::new(Mutex::new(PrinterStatus::Disconnected)),
        }
    }

    pub(crate) fn send_telemetry(&self, tel: TelemetryData) {
        let _ = self.tx.send(Action::Telemetry(tel));
    }

    pub(crate) fn update_status(&self, s: PrinterStatus) {
        if *self.printer_status.lock().unwrap() != s {
            *self.printer_status.lock().unwrap() = s.clone();
            let _ = self.tx.send(Action::StateChange(s));
        }
    }

    pub async fn start_event_loop(&mut self) {
        loop {
            if self.que.lock().unwrap().is_empty() {
                self.update_status(PrinterStatus::Idle);

                let listener = { self.event.lock().unwrap().listen() };
                listener.await;
            }

            let elem = self.que.lock().unwrap().pop_front();
            let que_len = self.que.lock().unwrap().len() as u32;

            if que_len != 0 {
                debug!("queue size: {}", que_len);
            }

            // do not send telemetry when we only update temp
            if que_len != 0 && elem != Some("M105".to_string()) {
                self.send_telemetry(TelemetryData::Progress(que_len));
            }

            match elem {
                None => {}
                Some(cmd) => {
                    // the received cmd here is expected to include no gcode comments (";") and have spaces trimmed
                    self.handle_presend_cmd(cmd.as_str());

                    let mut buffer: Vec<u8> = cmd.as_bytes().to_vec();
                    buffer.push(b'\n'); // Add newline character at the end of each command
                    if let Err(e) = self.port.write_all(&buffer) {
                        error!("Error while writing command: {}", e.to_string());
                        continue;
                    }
                    if let Err(e) = self.port.flush() {
                        error!("Error while writing command: {}", e.to_string());
                        continue;
                    }

                    if cmd != "M105" {
                        info!(">>>{}", cmd);
                    }

                    // handle an error message
                    if let Err(e) = self.read_until_ok().await {
                        error!("weve received an error response!");
                        error!("{}", e);

                        // when an error occurs clear queue
                        self.que.lock().unwrap().clear();
                    }
                }
            }
        }
    }

    pub async fn read_until_ok(&mut self) -> Result<Vec<String>, String> {
        let mut msgs: Vec<String> = Vec::new();
        let mut remainder = "".to_string();

        let mut timestamp = Instant::now();

        loop {
            while self.port.bytes_to_read().map_err(|e1| e1.to_string())? == 0 {
                if timestamp.elapsed().as_millis() > 5_000 {
                    warn!("Receive loop did not receive any message for more than 5sec!");
                    warn!("{:?}", msgs);
                    return Err("No response received".to_string());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }

            timestamp = Instant::now();
            let response = format!(
                "{}{}",
                remainder,
                self.read_port().map_err(|e| { e.to_string() })?
            );
            remainder = "".to_string();

            let mut lines: Vec<&str> = response.split("\n").collect();

            if !response.ends_with("\n") {
                remainder = lines.pop().unwrap_or("").to_string();
            }

            for line in lines {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                info!("<<<{}", line);

                self.handle_response(line);
                msgs.push(line.to_string());
            }

            // if printer is restarted there might be no 'ok' message
            if msgs.iter().any(|x| x.contains("ok") || x.contains("start")) {
                return Ok(msgs);
            } else if msgs
                .iter()
                .any(|x| x.contains("error") || x.contains("Error") || x.contains("Err"))
            {
                return Err(msgs.join(";").to_string());
            }
        }
    }

    fn read_port(&mut self) -> anyhow::Result<String> {
        let read_buf_size = self.port.bytes_to_read()?;
        let mut serial_buf: Vec<u8> = vec![0; read_buf_size as usize];
        let num_bytes_read = self.port.read(serial_buf.as_mut_slice())?;
        let response = String::from_utf8_lossy(&serial_buf[..num_bytes_read]).to_string();
        Ok(response)
    }
}
