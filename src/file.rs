use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Deserialize)]
pub enum FilamentType {
    PLA,
    PETG,
    ABS,
    TPU,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct GcodeFile {
    pub name: String,
    pub size: u64,
    pub last_modified: u128,
    pub filament_type: FilamentType,
}
