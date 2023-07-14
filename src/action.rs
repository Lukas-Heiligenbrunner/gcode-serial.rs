use crate::file::GcodeFile;
use crate::temperature::Temperature;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Display)]
pub enum PrinterStatus {
    Disconnected,
    Active,
    Idle,
    Errored,
}

#[derive(Clone, Serialize, Deserialize, Display)]
pub enum PrinterAction {
    Cancel,
    Pause,
    Resume,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TelemetryData {
    Temps(Temperature),
    Progress(u32),
    PercentDone(u32),
    MinsRemaining(u32),
    TotalCommandCount(u32),
    TargetExtruderTemp(u32),
    TargetBedTemp(u32),
    ZHeight(f32),
    MaxZHeight(f32),
    FanSpeed(f32),
    ActiveFile(Option<GcodeFile>),
    LastFileActive(GcodeFile),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Command {
    SetTemps(u16, u16),
    StartPrint(String),
    StopPrint,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Action {
    Telemetry(TelemetryData),
    StateChange(PrinterStatus),
    PrinterAction(PrinterAction),
    Command(Command),
}
