use super::LodesDataset;
use bamcensus_core::model::identifier::{Geoid, StateCode};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// A structured LODES query for a specific dataset configuration and state.
///
/// Holds query parameters as a typed record so the query can be inspected or modified
/// (such as substituting fallback candidate years) before materializing a download URI.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodesQuery {
    pub dataset: LodesDataset,
    pub state_code: StateCode,
}

impl LodesQuery {
    pub fn new(dataset: LodesDataset, geoid: &Geoid) -> Result<Self, String> {
        let state_code: StateCode = geoid.to_state().try_into()?;
        Ok(Self {
            dataset,
            state_code,
        })
    }

    pub fn year(&self) -> u64 {
        self.dataset.year()
    }

    pub fn with_year(&self, year: u64) -> Self {
        Self {
            dataset: self.dataset.with_year(year),
            state_code: self.state_code,
        }
    }

    pub fn to_uri(&self) -> Result<String, String> {
        self.dataset.create_uri_for_state(&self.state_code)
    }
}

impl Display for LodesQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({})",
            self.dataset,
            self.state_code.to_state_abbreviation()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LodesEdition, LodesJobType, WorkplaceSegment};

    #[test]
    fn test_lodes_query_with_year_updates_uri() {
        let geoid = Geoid::try_from("44").unwrap();
        let dataset = LodesDataset::WAC {
            edition: LodesEdition::Lodes8,
            job_type: LodesJobType::JT00,
            segment: WorkplaceSegment::S000,
            year: 2023,
        };

        let query = dataset.create_query(&geoid).unwrap();
        assert_eq!(query.year(), 2023);
        assert_eq!(
            query.to_uri().unwrap(),
            "https://lehd.ces.census.gov/data/lodes/LODES8/ri/wac/ri_wac_S000_JT00_2023.csv.gz"
        );

        let fallback_query = query.with_year(2022);
        assert_eq!(fallback_query.year(), 2022);
        assert_eq!(
            fallback_query.to_uri().unwrap(),
            "https://lehd.ces.census.gov/data/lodes/LODES8/ri/wac/ri_wac_S000_JT00_2022.csv.gz"
        );
    }

    #[test]
    fn test_lodes_query_defers_availability_validation() {
        // Alaska 2019 is statically marked unavailable in validate_availability,
        // but constructing the query succeeds so fallback can try earlier years (e.g., 2016).
        let ak = Geoid::try_from("02").unwrap();
        let dataset = LodesDataset::WAC {
            edition: LodesEdition::Lodes8,
            job_type: LodesJobType::JT00,
            segment: WorkplaceSegment::S000,
            year: 2019,
        };

        let query = LodesQuery::new(dataset, &ak).unwrap();
        assert!(query.to_uri().is_err());

        let fallback_query = query.with_year(2016);
        assert_eq!(
            fallback_query.to_uri().unwrap(),
            "https://lehd.ces.census.gov/data/lodes/LODES8/ak/wac/ak_wac_S000_JT00_2016.csv.gz"
        );
    }
}
