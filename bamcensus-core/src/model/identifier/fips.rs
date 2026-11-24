/// newtype structs to represent the sub-components of a GEOID.
use super::{
    geoid_type::GeoidType, has_geoid_string::HasGeoidString, has_geoid_type::HasGeoidType,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct State(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct County(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CountySubdivision(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Place(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CensusTract(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct BlockGroup(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Block(pub String);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CongressionalDistrict(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct StateLegislativeDistrictUpperChamber(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct StateLegislativeDistrictLowerChamber(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ZipCodeTabulationArea(pub u64);

impl HasGeoidType for State {
    fn geoid_type(&self) -> GeoidType {
        GeoidType::State
    }
}

impl HasGeoidType for County {
    fn geoid_type(&self) -> GeoidType {
        GeoidType::County
    }
}
impl HasGeoidType for CountySubdivision {
    fn geoid_type(&self) -> GeoidType {
        GeoidType::CountySubdivision
    }
}
impl HasGeoidType for Place {
    fn geoid_type(&self) -> GeoidType {
        GeoidType::Place
    }
}
impl HasGeoidType for CensusTract {
    fn geoid_type(&self) -> GeoidType {
        GeoidType::CensusTract
    }
}
impl HasGeoidType for BlockGroup {
    fn geoid_type(&self) -> GeoidType {
        GeoidType::BlockGroup
    }
}
impl HasGeoidType for Block {
    fn geoid_type(&self) -> GeoidType {
        GeoidType::Block
    }
}

impl HasGeoidString for State {
    fn geoid_string(&self) -> String {
        format!("{:02}", self.0)
    }
}

impl HasGeoidString for County {
    fn geoid_string(&self) -> String {
        format!("{:03}", self.0)
    }
}
impl HasGeoidString for CountySubdivision {
    fn geoid_string(&self) -> String {
        format!("{:05}", self.0)
    }
}
impl HasGeoidString for Place {
    fn geoid_string(&self) -> String {
        format!("{:05}", self.0)
    }
}
impl HasGeoidString for CensusTract {
    fn geoid_string(&self) -> String {
        format!("{:06}", self.0)
    }
}
impl HasGeoidString for BlockGroup {
    fn geoid_string(&self) -> String {
        self.0.to_string()
    }
}
impl HasGeoidString for Block {
    fn geoid_string(&self) -> String {
        self.0.clone()
    }
}
impl HasGeoidString for CongressionalDistrict {
    fn geoid_string(&self) -> String {
        format!("{:02}", self.0)
    }
}
impl HasGeoidString for StateLegislativeDistrictUpperChamber {
    fn geoid_string(&self) -> String {
        format!("{:03}", self.0)
    }
}
impl HasGeoidString for StateLegislativeDistrictLowerChamber {
    fn geoid_string(&self) -> String {
        format!("{:03}", self.0)
    }
}
impl HasGeoidString for ZipCodeTabulationArea {
    fn geoid_string(&self) -> String {
        format!("{:05}", self.0)
    }
}
