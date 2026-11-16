use std::fmt::Display;

use super::{fips, geoid::Geoid};
use clap::ValueEnum;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GeoidType {
    State,
    County,
    CountySubdivision,
    Place,
    CensusTract,
    BlockGroup,
    Block,
}

impl Display for GeoidType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GeoidType::State => String::from("state"),
            GeoidType::County => String::from("county"),
            GeoidType::CountySubdivision => String::from("county_subdivision"),
            GeoidType::Place => String::from("place"),
            GeoidType::CensusTract => String::from("census_tract"),
            GeoidType::BlockGroup => String::from("block_group"),
            GeoidType::Block => String::from("block"),
        };
        write!(f, "{s}")
    }
}

impl GeoidType {
    pub fn geoid_from_str(&self, value: &str) -> Result<Geoid, String> {
        let value_len = value.len();
        match self {
            GeoidType::State => {
                if value_len != 2 {
                    Err(format!(
                        "for state geoid, expected 2-digit value, found: {value}"
                    ))
                } else {
                    self.geoid_from_slice_of_strings(&[value.to_string()])
                }
            }
            GeoidType::County => {
                if value_len != 5 {
                    Err(format!(
                        "for county geoid, expected 5-digit value, found: {value}"
                    ))
                } else {
                    self.geoid_from_slice_of_strings(&[
                        value[0..2].to_string(),
                        value[2..5].to_string(),
                    ])
                }
            }
            GeoidType::CountySubdivision => {
                if value_len != 10 {
                    Err(format!(
                        "for county subdivision geoid, expected 10-digit value, found: {value}"
                    ))
                } else {
                    self.geoid_from_slice_of_strings(&[
                        value[0..2].to_string(),
                        value[2..5].to_string(),
                        value[5..10].to_string(),
                    ])
                }
            }
            GeoidType::Place => {
                if value_len != 7 {
                    Err(format!(
                        "for place geoid, expected 7-digit value, found: {value}"
                    ))
                } else {
                    self.geoid_from_slice_of_strings(&[
                        value[0..2].to_string(),
                        value[2..7].to_string(),
                    ])
                }
            }
            GeoidType::CensusTract => {
                if value_len != 11 {
                    Err(format!(
                        "for census tract geoid, expected 11-digit value, found: {value}"
                    ))
                } else {
                    self.geoid_from_slice_of_strings(&[
                        value[0..2].to_string(),
                        value[2..5].to_string(),
                        value[5..11].to_string(),
                    ])
                }
            }
            GeoidType::BlockGroup => {
                if value_len != 12 {
                    Err(format!(
                        "for block group geoid, expected 12-digit value, found: {value}"
                    ))
                } else {
                    self.geoid_from_slice_of_strings(&[
                        value[0..2].to_string(),
                        value[2..5].to_string(),
                        value[5..11].to_string(),
                        value[11..12].to_string(),
                    ])
                }
            }
            GeoidType::Block => {
                if value_len != 15 && value_len != 16 {
                    Err(format!(
                        "for block geoid, expected 15 or 16-digit value, found: {value} with length {value_len}"
                    ))
                } else {
                    self.geoid_from_slice_of_strings(&[
                        value[0..2].to_string(),
                        value[2..5].to_string(),
                        value[5..11].to_string(),
                        value[11..].to_string(),
                    ])
                }
            }
        }
    }
    pub fn geoid_from_slice_of_strings(&self, vals: &[String]) -> Result<Geoid, String> {
        match self {
            GeoidType::State => {
                let arr = as_usizes(vals)?;
                if arr.len() != 1 {
                    Err(format!(
                        "for state-level query, expected 1 geoid column, found: {}",
                        arr.into_iter().join(",")
                    ))
                } else {
                    Ok(Geoid::State(fips::State(arr[0])))
                }
            }
            GeoidType::County => {
                let arr = as_usizes(vals)?;
                if arr.len() != 2 {
                    Err(format!(
                        "for county-level query, expected 2 geoid columns, found: {}",
                        arr.into_iter().join(",")
                    ))
                } else {
                    Ok(Geoid::County(fips::State(arr[0]), fips::County(arr[1])))
                }
            }
            GeoidType::CountySubdivision => {
                let arr = as_usizes(vals)?;
                if arr.len() != 3 {
                    Err(format!(
                        "for county subdivision-level query, expected 3 geoid columns, found: {}",
                        arr.into_iter().join(",")
                    ))
                } else {
                    Ok(Geoid::CountySubdivision(
                        fips::State(arr[0]),
                        fips::County(arr[1]),
                        fips::CountySubdivision(arr[2]),
                    ))
                }
            }
            GeoidType::Place => {
                let arr = as_usizes(vals)?;
                if arr.len() != 2 {
                    Err(format!(
                        "for place-level query, expected 2 geoid columns, found: {}",
                        arr.into_iter().join(",")
                    ))
                } else {
                    Ok(Geoid::Place(fips::State(arr[0]), fips::Place(arr[1])))
                }
            }
            GeoidType::CensusTract => {
                let arr = as_usizes(vals)?;
                if arr.len() != 3 {
                    Err(format!(
                        "for census tract-level query, expected 3 geoid column, found: {}",
                        arr.into_iter().join(",")
                    ))
                } else {
                    Ok(Geoid::CensusTract(
                        fips::State(arr[0]),
                        fips::County(arr[1]),
                        fips::CensusTract(arr[2]),
                    ))
                }
            }
            GeoidType::BlockGroup => {
                let arr = as_usizes(vals)?;
                let arr_len = arr.len();
                if arr_len != 4 {
                    Err(format!(
                        "for block group-level query, expected 4 geoid columns, found: {} with length {}",
                        arr.into_iter().join(","),
                        arr_len
                    ))
                } else {
                    Ok(Geoid::BlockGroup(
                        fips::State(arr[0]),
                        fips::County(arr[1]),
                        fips::CensusTract(arr[2]),
                        fips::BlockGroup(arr[3]),
                    ))
                }
            }
            GeoidType::Block => {
                let arr = as_usizes(vals)?;
                if arr.len() != 4 {
                    Err(format!(
                        "for block-level query, expected 4 geoid columns, found: {}",
                        arr.into_iter().join(",")
                    ))
                } else {
                    Ok(Geoid::Block(
                        fips::State(arr[0]),
                        fips::County(arr[1]),
                        fips::CensusTract(arr[2]),
                        fips::Block(vals[3].clone()),
                    ))
                }
            }
        }
    }
}

/// helper function to convert a slice of strings into u64s used to build fips::* values.
fn as_usizes(arr: &[String]) -> Result<Vec<u64>, String> {
    arr.iter()
        .map(|v| {
            let v_u64 = v.parse::<u64>().map_err(|e| {
                format!(
                    "raw geoid value should be a string wrapping an integer, found '{v}'. error: {e}"
                )
            })?;
            Ok(v_u64)
        })
        .collect::<Result<Vec<u64>, String>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::identifier::HasGeoidString;

    #[test]
    fn block_geoid_preserves_leading_zeroes() {
        let value = "080310001000001";
        assert_eq!(
            GeoidType::Block
                .geoid_from_str(value)
                .unwrap()
                .geoid_string(),
            value
        );
    }
}
