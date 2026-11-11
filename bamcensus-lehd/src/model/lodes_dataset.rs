use super::{
    LodesEdition, LodesJobType, LodesQuery, OdPart, WorkplaceSegment, BASE_URL, LATEST_YEAR,
};
use bamcensus_core::model::identifier::{Geoid, GeoidType, StateCode};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LodesDataset {
    OD {
        edition: LodesEdition,
        job_type: LodesJobType,
        od_part: OdPart,
        year: u64,
    },
    RAC {
        edition: LodesEdition,
        job_type: LodesJobType,
        segment: WorkplaceSegment,
        year: u64,
    },
    WAC {
        edition: LodesEdition,
        job_type: LodesJobType,
        segment: WorkplaceSegment,
        year: u64,
    },
}

impl Default for LodesDataset {
    fn default() -> Self {
        let year = LATEST_YEAR;
        Self::WAC {
            edition: LodesEdition::default(),
            job_type: LodesJobType::default(),
            segment: WorkplaceSegment::default(),
            year,
        }
    }
}

impl Display for LodesDataset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl LodesDataset {
    pub fn description(&self) -> String {
        match self {
            LodesDataset::OD {
                edition,
                job_type,
                od_part,
                year,
            } => {
                format!("{year} {edition} {od_part} Origin-Destination data, {job_type} job totals are associated with both a home Census Block and a work Census Block")
            }
            LodesDataset::RAC {
                edition: _,
                job_type: _,
                segment: _,
                year: _,
            } => String::from(
                "Residence Area Characteristic data, jobs are totaled by home Census Block",
            ),
            LodesDataset::WAC {
                edition,
                job_type,
                segment,
                year,
            } => format!(
                "{year} {edition} {segment} Workplace Area Characteristic data, {job_type} jobs are totaled by work Census Block"
            ),
        }
    }

    pub fn dataset_directory(&self) -> String {
        match self {
            LodesDataset::OD {
                edition: _,
                job_type: _,
                od_part: _,
                year: _,
            } => String::from("od"),
            LodesDataset::RAC {
                edition: _,
                job_type: _,
                segment: _,
                year: _,
            } => String::from("rac"),
            LodesDataset::WAC {
                edition: _,
                job_type: _,
                segment: _,
                year: _,
            } => String::from("wac"),
        }
    }

    pub fn year(&self) -> u64 {
        match self {
            LodesDataset::OD { year, .. }
            | LodesDataset::RAC { year, .. }
            | LodesDataset::WAC { year, .. } => *year,
        }
    }

    pub fn with_year(&self, year: u64) -> Self {
        match *self {
            LodesDataset::OD {
                edition,
                job_type,
                od_part,
                ..
            } => LodesDataset::OD {
                edition,
                job_type,
                od_part,
                year,
            },
            LodesDataset::RAC {
                edition,
                job_type,
                segment,
                ..
            } => LodesDataset::RAC {
                edition,
                job_type,
                segment,
                year,
            },
            LodesDataset::WAC {
                edition,
                job_type,
                segment,
                ..
            } => LodesDataset::WAC {
                edition,
                job_type,
                segment,
                year,
            },
        }
    }

    pub fn create_query(&self, geoid: &Geoid) -> Result<LodesQuery, String> {
        LodesQuery::new(*self, geoid)
    }

    /// creates a URI to a LODES datasets based on the directory and file
    /// naming conventions described in the LODESTechDoc8.1.pdf file.
    /// see <https://lehd.ces.census.gov/data/lodes/LODES8/LODESTechDoc8.1.pdf>
    pub fn create_uri(&self, geoid: &Geoid) -> Result<String, String> {
        let sc: StateCode = geoid.to_state().try_into()?;
        self.create_uri_for_state(&sc)
    }

    pub fn create_uri_for_state(&self, sc: &StateCode) -> Result<String, String> {
        let state_code = sc.to_state_abbreviation();
        match self {
            LodesDataset::OD {
                edition,
                job_type,
                od_part,
                year,
            } => {
                validate_availability(*year, sc)?;
                let filename = format!(
                    "{}_od_{}_{}_{}.csv.gz",
                    state_code.to_lowercase(),
                    od_part,
                    job_type,
                    year
                );
                let uri = format!(
                    "{}/{}/{}/{}/{}",
                    BASE_URL,
                    edition,
                    state_code.to_lowercase(),
                    self.dataset_directory(),
                    filename
                );
                Ok(uri)
            }
            LodesDataset::RAC {
                edition,
                job_type,
                segment,
                year,
            } => {
                let filename = format!(
                    "{}_rac_{}_{}_{}.csv.gz",
                    state_code.to_lowercase(),
                    segment,
                    job_type,
                    year
                );
                let uri = format!(
                    "{}/{}/{}/{}/{}",
                    BASE_URL,
                    edition,
                    state_code.to_lowercase(),
                    self.dataset_directory(),
                    filename
                );
                Ok(uri)
            }
            LodesDataset::WAC {
                edition,
                job_type,
                segment,
                year,
            } => {
                validate_availability(*year, sc)?;
                let filename = format!(
                    "{}_wac_{}_{}_{}.csv.gz",
                    state_code.to_lowercase(),
                    segment,
                    job_type,
                    year
                );
                let uri = format!(
                    "{}/{}/{}/{}/{}",
                    BASE_URL,
                    edition,
                    state_code.to_lowercase(),
                    self.dataset_directory(),
                    filename
                );
                Ok(uri)
            }
        }
    }

    pub fn output_filename(&self, wildcard: &Option<GeoidType>) -> String {
        match self {
            LodesDataset::OD {
                edition,
                job_type,
                od_part,
                year,
            } => {
                let out_res = wildcard.unwrap_or(GeoidType::Block);
                format!("{edition}_od_{year}_{job_type}_{od_part}_{out_res}.csv")
            }
            LodesDataset::RAC {
                edition,
                job_type,
                segment,
                year,
            } => {
                let out_res = wildcard.unwrap_or(GeoidType::Block);
                format!("{edition}_rac_{year}_{job_type}_{segment}_{out_res}.csv")
            }
            LodesDataset::WAC {
                edition,
                job_type,
                segment,
                year,
            } => {
                let out_res = wildcard.unwrap_or(GeoidType::Block);
                format!("{edition}_wac_{year}_{job_type}_{segment}_{out_res}.csv")
            }
        }
    }

    /// LODES editions correspond to specific TIGER/Lines datasets. see
    /// [`LodesEdition::tiger_year`] for details. this year value should
    /// be used when downloading complimentary TIGER/Lines datasets.
    pub fn tiger_year(&self) -> u64 {
        match self {
            LodesDataset::OD {
                edition,
                job_type: _,
                od_part: _,
                year: _,
            } => edition.tiger_year(),
            LodesDataset::RAC {
                edition,
                job_type: _,
                segment: _,
                year: _,
            } => edition.tiger_year(),
            LodesDataset::WAC {
                edition,
                job_type: _,
                segment: _,
                year: _,
            } => edition.tiger_year(),
        }
    }
}

/// as outlined in the tech doc, some states do not have WAC or OD data for certain years
/// see <https://lehd.ces.census.gov/data/lodes/LODES8/LODESTechDoc8.1.pdf>
fn validate_availability(year: u64, state_code: &StateCode) -> Result<(), String> {
    let err = || {
        Err(format!(
            "WAC is not available in {} for {} (code {})",
            year,
            state_code.to_full_name(),
            state_code.to_fips_string()
        ))
    };
    match (year, state_code) {
        (2002, StateCode::Arkansas) => err(),
        (2002, StateCode::NewHampshire) => err(),
        (y, StateCode::Arizona) if in_range_exclusive(y, 2002, 2003) => err(),
        (y, StateCode::Mississippi) if in_range_exclusive(y, 2002, 2003) => err(),
        (y, StateCode::DistrictOfColumbia) if in_range_exclusive(y, 2002, 2009) => err(),
        (y, StateCode::Massachusetts) if in_range_exclusive(y, 2002, 2010) => err(),
        (y, StateCode::Alaska) if in_range_exclusive(y, 2017, 2020) => err(),
        (y, StateCode::Arkansas) if in_range_exclusive(y, 2019, 2020) => err(),
        (y, StateCode::Mississippi) if in_range_exclusive(y, 2019, 2020) => err(),
        _ => Ok(()),
    }
}

fn in_range_exclusive(y: u64, min: u64, max: u64) -> bool {
    min <= y && y <= max
}
