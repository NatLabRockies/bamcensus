use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Origin-Destination data, jobs totals are associated with both a
/// home Census Block and a work Census Block
///
/// the columns in a downloaded LODES OD dataset, or, one of the
/// parameters of a WAC dataset filename.
#[derive(Default, ValueEnum, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkplaceSegment {
    #[default]
    S000,
    SA01,
    SA02,
    SA03,
    SE01,
    SE02,
    SE03,
    SI01,
    SI02,
    SI03,
}

impl Display for WorkplaceSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl WorkplaceSegment {
    pub fn description(&self) -> String {
        match self {
            WorkplaceSegment::S000 => String::from("Num Total number of jobs"),
            WorkplaceSegment::SA01 => {
                String::from("Num Number of jobs of workers age 29 or younger9")
            }
            WorkplaceSegment::SA02 => String::from("Num Number of jobs for workers age 30 to 549"),
            WorkplaceSegment::SA03 => {
                String::from("Num Number of jobs for workers age 55 or older 9")
            }
            WorkplaceSegment::SE01 => {
                String::from("Num Number of jobs with earnings $1250/month or less")
            }
            WorkplaceSegment::SE02 => {
                String::from("Num Number of jobs with earnings $1251/month to $3333/month")
            }
            WorkplaceSegment::SE03 => {
                String::from("Num Number of jobs with earnings greater than $3333/month")
            }
            WorkplaceSegment::SI01 => {
                String::from("Num Number of jobs in Goods Producing industry sectors")
            }
            WorkplaceSegment::SI02 => String::from(
                "Num Number of jobs in Trade, Transportation, and Utilities industry sectors",
            ),
            WorkplaceSegment::SI03 => {
                String::from("Num Number of jobs in All Other Services industry sectors")
            }
        }
    }
}
