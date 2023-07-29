use crate::models::file::{FinishedPrint, GcodeFile};
use crate::models::temperature::Temperature;
use serde::{Deserialize, Serialize};
use strum::Display;

/// Main struct to communicate with the lib
#[derive(Clone, Serialize, Deserialize)]
pub enum Action {
    /// telemetry data from the lib
    Telemetry(TelemetryData),
    /// The state of the printer changed (eg. idle or active)
    StateChange(PrinterStatus),
    /// a action from the printer (eg. cancel or pause)
    PrinterAction(PrinterAction),
    /// send a command to the lib
    Command(Command),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Display)]
pub enum PrinterStatus {
    Disconnected,
    Active,
    Idle,
    Errored,
}

/// Action sent by printer
#[derive(Clone, Serialize, Deserialize, Display)]
pub enum PrinterAction {
    Cancel,
    Pause,
    Resume,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TelemetryData {
    /// Hotend and Bed Temperature telemtry
    Temps(Temperature),
    /// commands left in Que
    Progress(u32),
    /// Currently only used by sd-card prints (percentage completed)
    PercentDone(u32),
    /// Currently only used by sd-card prints (Minutes left to print)
    MinsRemaining(u32),
    /// total number of commands of active print
    TotalCommandCount(u32),
    /// target extruder temp changed
    TargetExtruderTemp(u32),
    /// target bed temp changed
    TargetBedTemp(u32),
    /// zHeight changed
    ZHeight(f32),
    /// Maximum ZHeight of the current print
    MaxZHeight(f32),
    /// Fan speed changed
    FanSpeed(f32),
    /// Active print file changed either file or none
    ActiveFileChange(Option<GcodeFile>),
    /// Finished print event
    PrintFinished(FinishedPrint),
}

/// Send an Action command to the lib
#[derive(Clone, Serialize, Deserialize)]
pub enum Command {
    /// Set target temps (bed, extruder)
    SetTemps(u16, u16),
    /// Start printing a file given by path
    StartPrint(String),
    /// stop currently active print
    StopPrint,
}
