use crate::models::action::{Action, PrinterStatus};
use crate::serial::serial::Serial;
use log::warn;
use std::time::Duration;

impl Serial {
    pub fn start_temp_interval(&self) {
        let que = self.que.clone();
        let printerstatus = self.printer_status.clone();
        let event = self.event.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let mut alive_counter: u32 = 0;

            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;

                // check if the next action isn't already a temp poll
                // todo recheck with some unix timestamp when the last temp poll was
                if que.lock().unwrap().front().unwrap_or(&"".to_string()) != &"M105".to_string() {
                    if *printerstatus.lock().unwrap() == PrinterStatus::Disconnected {
                        if que.lock().unwrap().len() > 10 {
                            *printerstatus.lock().unwrap() = PrinterStatus::Active;
                            let _ = tx.send(Action::StateChange(PrinterStatus::Active));
                        } else {
                            *printerstatus.lock().unwrap() = PrinterStatus::Idle;
                            let _ = tx.send(Action::StateChange(PrinterStatus::Idle));
                        }
                    }

                    alive_counter = 0;
                    que.lock().unwrap().push_front("M105".to_string());
                } else {
                    if alive_counter >= 4 {
                        warn!("There seems to be no connection to printer");
                        // this might be also triggered when a print is started at the heating process await
                        if *printerstatus.lock().unwrap() != PrinterStatus::Disconnected {
                            *printerstatus.lock().unwrap() = PrinterStatus::Disconnected;
                            let _ = tx.send(Action::StateChange(PrinterStatus::Disconnected));
                        }
                    } else {
                        alive_counter += 1;
                    }
                }
                event.lock().unwrap().notify(42);
            }
        });
    }
}
