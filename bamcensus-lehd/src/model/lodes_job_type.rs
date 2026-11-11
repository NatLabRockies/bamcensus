use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

// scripts for downloading LODES datasets
// see https://lehd.ces.census.gov/data/lodes/LODES8/LODESTechDoc8.1.pdf

#[derive(Default, ValueEnum, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LodesJobType {
    #[default]
    JT00,
    JT01,
    JT02,
    JT03,
    JT04,
    JT05,
}

impl LodesJobType {
    pub fn description(&self) -> String {
        match self {
            LodesJobType::JT00 => String::from("All Jobs"),
            LodesJobType::JT01 => String::from("Primary Jobs"),
            LodesJobType::JT02 => String::from("All Private Jobs"),
            LodesJobType::JT03 => String::from("Private Primary Jobs"),
            LodesJobType::JT04 => String::from("All Federal Jobs"),
            LodesJobType::JT05 => String::from("Federal Primary Jobs"),
        }
    }
}

impl Display for LodesJobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
