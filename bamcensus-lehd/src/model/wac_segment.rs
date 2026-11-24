use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, ValueEnum, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "UPPERCASE")]
pub enum WacSegment {
    #[default]
    C000,
    CA01,
    CA02,
    CA03,
    CE01,
    CE02,
    CE03,
    CNS01,
    CNS02,
    CNS03,
    CNS04,
    CNS05,
    CNS06,
    CNS07,
    CNS08,
    CNS09,
    CNS10,
    CNS11,
    CNS12,
    CNS13,
    CNS14,
    CNS15,
    CNS16,
    CNS17,
    CNS18,
    CNS19,
    CNS20,
    CR01,
    CR02,
    CR03,
    CR04,
    CR05,
    CR07,
    CT01,
    CT02,
    CD01,
    CD02,
    CD03,
    CD04,
    CS01,
    CS02,
}

impl TryFrom<&str> for WacSegment {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "C000" => Ok(Self::C000),
            "CA01" => Ok(Self::CA01),
            "CA02" => Ok(Self::CA02),
            "CA03" => Ok(Self::CA03),
            "CE01" => Ok(Self::CE01),
            "CE02" => Ok(Self::CE02),
            "CE03" => Ok(Self::CE03),
            "CNS01" => Ok(Self::CNS01),
            "CNS02" => Ok(Self::CNS02),
            "CNS03" => Ok(Self::CNS03),
            "CNS04" => Ok(Self::CNS04),
            "CNS05" => Ok(Self::CNS05),
            "CNS06" => Ok(Self::CNS06),
            "CNS07" => Ok(Self::CNS07),
            "CNS08" => Ok(Self::CNS08),
            "CNS09" => Ok(Self::CNS09),
            "CNS10" => Ok(Self::CNS10),
            "CNS11" => Ok(Self::CNS11),
            "CNS12" => Ok(Self::CNS12),
            "CNS13" => Ok(Self::CNS13),
            "CNS14" => Ok(Self::CNS14),
            "CNS15" => Ok(Self::CNS15),
            "CNS16" => Ok(Self::CNS16),
            "CNS17" => Ok(Self::CNS17),
            "CNS18" => Ok(Self::CNS18),
            "CNS19" => Ok(Self::CNS19),
            "CNS20" => Ok(Self::CNS20),
            "CR01" => Ok(Self::CR01),
            "CR02" => Ok(Self::CR02),
            "CR03" => Ok(Self::CR03),
            "CR04" => Ok(Self::CR04),
            "CR05" => Ok(Self::CR05),
            "CR07" => Ok(Self::CR07),
            "CT01" => Ok(Self::CT01),
            "CT02" => Ok(Self::CT02),
            "CD01" => Ok(Self::CD01),
            "CD02" => Ok(Self::CD02),
            "CD03" => Ok(Self::CD03),
            "CD04" => Ok(Self::CD04),
            "CS01" => Ok(Self::CS01),
            "CS02" => Ok(Self::CS02),
            _ => Err(format!("unknown WAC Segment {value}")),
        }
    }
}

impl Display for WacSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl WacSegment {
    pub fn description(&self) -> String {
        match self {
            Self::C000 =>String::from("Total number of jobs"),
            Self::CA01 =>String::from("Number of jobs for workers age 29 or younger 9"),
            Self::CA02 =>String::from("Number of jobs for workers age 30 to 549"),
            Self::CA03 =>String::from("Number of jobs for workers age 55 or older 9"),
            Self::CE01 =>String::from("Number of jobs with earnings $1250/month or less"),
            Self::CE02 =>String::from("Number of jobs with earnings $1251/month to $3333/month"),
            Self::CE03 =>String::from("Number of jobs with earnings greater than $3333/month"),
            Self::CNS01 =>String::from("Number of jobs in NAICS sector 11 (Agriculture, Forestry, Fishing and Hunting)"),
            Self::CNS02 =>String::from("Number of jobs in NAICS sector 21 (Mining, Quarrying, and Oil and Gas Extraction)"),
            Self::CNS03 =>String::from("Number of jobs in NAICS sector 22 (Utilities)"),
            Self::CNS04 =>String::from("Number of jobs in NAICS sector 23 (Construction)"),
            Self::CNS05 =>String::from("Number of jobs in NAICS sector 31-33 (Manufacturing)"),
            Self::CNS06 =>String::from("Number of jobs in NAICS sector 42 (Wholesale Trade)"),
            Self::CNS07 =>String::from("Number of jobs in NAICS sector 44-45 (Retail Trade)"),
            Self::CNS08 =>String::from("Number of jobs in NAICS sector 48-49 (Transportation and Warehousing)"),
            Self::CNS09 =>String::from("Number of jobs in NAICS sector 51 (Information)"),
            Self::CNS10 =>String::from("Number of jobs in NAICS sector 52 (Finance and Insurance)"),
            Self::CNS11 =>String::from("Number of jobs in NAICS sector 53 (Real Estate and Rental and Leasing)"),
            Self::CNS12 =>String::from("Number of jobs in NAICS sector 54 (Professional, Scientific, and Technical Services)"),
            Self::CNS13 =>String::from("Number of jobs in NAICS sector 55 (Management of Companies and Enterprises)"),
            Self::CNS14 =>String::from("Number of jobs in NAICS sector 56 (Administrative and Support and Waste Management and Remediation Services)"),
            Self::CNS15 =>String::from("Number of jobs in NAICS sector 61 (Educational Services)"),
            Self::CNS16 =>String::from("Number of jobs in NAICS sector 62 (Health Care and Social Assistance)"),
            Self::CNS17 =>String::from("Number of jobs in NAICS sector 71 (Arts, Entertainment, and Recreation)"),
            Self::CNS18 =>String::from("Number of jobs in NAICS sector 72 (Accommodation and Food Services)"),
            Self::CNS19 =>String::from("Number of jobs in NAICS sector 81 (Other Services [except Public Administration])"),
            Self::CNS20 =>String::from("Number of jobs in NAICS sector 92 (Public Administration)"),
            Self::CR01 =>String::from("Number of jobs for workers with Race: White, Alone10"),
            Self::CR02 =>String::from("Number of jobs for workers with Race: Black or African American Alone10"),
            Self::CR03 =>String::from("Number of jobs for workers with Race: American Indian or Alaska Native Alone10"),
            Self::CR04 =>String::from("Number of jobs for workers with Race: Asian Alone10"),
            Self::CR05 =>String::from("Number of jobs for workers with Race: Native Hawaiian or Other Pacific Islander Alone10"),
            Self::CR07 =>String::from("Number of jobs for workers with Race: Two or More Race Groups 10"),
            Self::CT01 =>String::from("Number of jobs for workers with Ethnicity: Not Hispanic or Latino 10"),
            Self::CT02 =>String::from("Number of jobs for workers with Ethnicity: Hispanic or Latino 10"),
            Self::CD01 =>String::from("Number of jobs for workers with Educational Attainment: Less than high school 10,11"),
            Self::CD02 =>String::from("Number of jobs for workers with Educational Attainment: High school or equivalent, no college 10,11"),
            Self::CD03 =>String::from("Number of jobs for workers with Educational Attainment: Some college or Associate degree 10,11"),
            Self::CD04 =>String::from("Number of jobs for workers with Educational Attainment: Bachelor’s degree or advanced degree10,11"),
            Self::CS01 =>String::from("Number of jobs for workers with Sex: Male 10"),
            Self::CS02 =>String::from("Number of jobs for workers with Sex: Female10"),
        }
    }

    pub fn naics(&self) -> Option<Vec<u64>> {
        match self {
            Self::CNS01 => Some(vec![11]),
            Self::CNS02 => Some(vec![21]),
            Self::CNS03 => Some(vec![22]),
            Self::CNS04 => Some(vec![23]),
            Self::CNS05 => Some(vec![31, 32, 33]),
            Self::CNS06 => Some(vec![42]),
            Self::CNS07 => Some(vec![44, 45]),
            Self::CNS08 => Some(vec![48, 49]),
            Self::CNS09 => Some(vec![51]),
            Self::CNS10 => Some(vec![52]),
            Self::CNS11 => Some(vec![53]),
            Self::CNS12 => Some(vec![54]),
            Self::CNS13 => Some(vec![55]),
            Self::CNS14 => Some(vec![56]),
            Self::CNS15 => Some(vec![61]),
            Self::CNS16 => Some(vec![62]),
            Self::CNS17 => Some(vec![71]),
            Self::CNS18 => Some(vec![72]),
            Self::CNS19 => Some(vec![81]),
            Self::CNS20 => Some(vec![92]),
            _ => None,
        }
    }
}
