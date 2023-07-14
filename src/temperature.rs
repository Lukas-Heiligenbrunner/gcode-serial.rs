use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Temperature {
    pub timestamp: u64,
    pub bed_temp: f32,
    pub ex_temp: f32,
}

impl Display for Temperature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bed: ({}), Extruder: ({})", self.bed_temp, self.ex_temp)
    }
}

impl Default for &Temperature {
    fn default() -> Self {
        &Temperature {
            timestamp: 0,
            bed_temp: 0.0,
            ex_temp: 0.0,
        }
    }
}
