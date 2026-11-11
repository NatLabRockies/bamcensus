use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Deserialize, Serialize, ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OdPart {
    Main,
    Aux,
}

impl Display for OdPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OdPart::Main => write!(f, "main"),
            OdPart::Aux => write!(f, "aux"),
        }
    }
}
