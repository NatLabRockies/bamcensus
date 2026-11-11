use super::LodesDataset;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Deserialize, Serialize, ValueEnum, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LodesEdition {
    Lodes6,
    Lodes7,
    #[default]
    Lodes8,
}

impl Display for LodesEdition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LodesEdition::Lodes6 => write!(f, "LODES6"),
            LodesEdition::Lodes7 => write!(f, "LODES7"),
            LodesEdition::Lodes8 => write!(f, "LODES8"),
        }
    }
}

impl LodesEdition {
    /// matches the LODES edition and requested dataset year to the
    /// corresponding TIGER/Lines data year, preventing mismatches.
    ///
    /// Geography Vintage
    /// LODES Version 8.1 is based on 2021 TIGER/Line shapefiles. The data are enumerated with 2020 census
    /// blocks. LODES Version 7 and 6 used 2010 census blocks. Basic information on 2020 census blocks can be
    /// found at www.census.gov/geographies/reference-files/time-series/geo/block-assignment-files.html.
    /// General information on the relationships between 2010 census blocks and 2020 census blocks can be
    /// found at www.census.gov/geographies/reference-files/time-series/geo/relationship-files.html. The
    /// methods used to translate historical data into 2020 census blocks can be found at
    /// lehd.ces.census.gov/doc/help/onthemap/OnTheMap2020Geography.pdf.
    pub fn tiger_year(&self) -> u64 {
        match self {
            LodesEdition::Lodes6 => 2010,
            LodesEdition::Lodes7 => 2010,
            LodesEdition::Lodes8 => 2021,
        }
    }

    pub fn create_url(
        &self,
        state_code: &str,
        lodes_dataset: &LodesDataset,
        filename: &String,
    ) -> String {
        format!(
            "{}/{}/{}/{}/{}",
            super::BASE_URL,
            self,
            state_code,
            lodes_dataset.to_string().to_lowercase(),
            filename
        )
    }
}
