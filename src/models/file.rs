use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Deserialize)]
pub struct GcodeFile {
    pub name: String,
    pub size: u64,
    pub last_modified: u128,
    pub start_time: u128,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct FinishedPrint {
    pub name: String,
    pub size: u64,
    pub last_modified: u128,
    pub start_time: u128,
    pub finish_time: u128,
}
