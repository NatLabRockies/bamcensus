use super::{fips, GeoidType, HasGeoidString, StateCode};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Geoid {
    State(fips::State),
    County(fips::State, fips::County),
    CountySubdivision(fips::State, fips::County, fips::CountySubdivision),
    Place(fips::State, fips::Place),
    CensusTract(fips::State, fips::County, fips::CensusTract),
    BlockGroup(
        fips::State,
        fips::County,
        fips::CensusTract,
        fips::BlockGroup,
    ),
    Block(fips::State, fips::County, fips::CensusTract, fips::Block),
}

impl TryFrom<&str> for Geoid {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.len() {
            2 => GeoidType::State.geoid_from_str(value),
            5 => GeoidType::County.geoid_from_str(value),
            7 => GeoidType::Place.geoid_from_str(value),
            10 => GeoidType::CountySubdivision.geoid_from_str(value),
            11 => GeoidType::CensusTract.geoid_from_str(value),
            12 => GeoidType::BlockGroup.geoid_from_str(value),
            x if x == 15 || x == 16 => GeoidType::Block.geoid_from_str(value),
            x => Err(format!("unsupported GEOID type with length {x}: {value}")),
        }
    }
}

// todo:
// - Geoid methods to unpack/pack between types (Geoid::County.to_state())

impl Geoid {
    /// generates all state level Geoids for the U.S.
    pub fn all_states() -> Vec<Geoid> {
        StateCode::ALL
            .iter()
            .map(|sc| {
                let s: fips::State = (*sc).into();
                Geoid::State(s)
            })
            .collect_vec()
    }

    pub fn geoid_type(&self) -> GeoidType {
        match self {
            Geoid::State(_) => GeoidType::State,
            Geoid::County(_, _) => GeoidType::County,
            Geoid::CountySubdivision(_, _, _) => GeoidType::CountySubdivision,
            Geoid::Place(_, _) => GeoidType::Place,
            Geoid::CensusTract(_, _, _) => GeoidType::CensusTract,
            Geoid::BlockGroup(_, _, _, _) => GeoidType::BlockGroup,
            Geoid::Block(_, _, _, _) => GeoidType::Block,
        }
    }

    pub fn variant_name(&self) -> String {
        self.geoid_type().to_string()
    }

    /// manipulates this GEOID via truncation to transform it's GEOID type.
    ///
    /// GEOID is a hierarchical numeric identifier. we can truncate the values
    /// in order to reach a higher/larger geographic representation. this method
    /// supports that operation, where this Geoid instance will be truncated to
    /// transform it into some other GeoidType.
    ///
    /// # Examples
    ///
    /// converts GEOID 08059009838 (TRACT) to 08059 (COUNTY).
    ///
    /// ```rust
    /// use bamcensus_core::model::identifier::{Geoid, GeoidType, fips};
    /// let geoid = Geoid::CensusTract(
    ///     fips::State(8),         // 08     Colorado
    ///     fips::County(59),       // 059    Jefferson
    ///     fips::CensusTract(9838) // 009838
    /// );
    /// let result = geoid.truncate_geoid_to_type(&GeoidType::County).unwrap();
    /// assert_eq!(result, Geoid::County(fips::State(8), fips::County(59)))
    /// ```
    pub fn truncate_geoid_to_type(&self, target: &GeoidType) -> Result<Geoid, String> {
        fn _err(src: &GeoidType, dst: &GeoidType) -> String {
            format!("{dst} not a parent type of {src}, cannot truncate geoid.")
        }
        match (self, target) {
            (Geoid::State(_), GeoidType::State) => Ok(self.clone()),
            (Geoid::State(_), _) => Err(_err(&self.geoid_type(), target)),
            (Geoid::County(s, _), GeoidType::State) => Ok(Geoid::State(*s)),
            (Geoid::County(_, _), GeoidType::County) => Ok(self.clone()),
            (Geoid::County(_, _), _) => Err(_err(&self.geoid_type(), target)),
            (Geoid::CountySubdivision(s, _, _), GeoidType::State) => Ok(Geoid::State(*s)),
            (Geoid::CountySubdivision(s, c, _), GeoidType::County) => Ok(Geoid::County(*s, *c)),
            (Geoid::CountySubdivision(_, _, _), GeoidType::CountySubdivision) => Ok(self.clone()),
            (Geoid::CountySubdivision(_, _, _), _) => Err(_err(&self.geoid_type(), target)),
            (Geoid::Place(s, _), GeoidType::State) => Ok(Geoid::State(*s)),
            (Geoid::Place(_, _), _) => Err(_err(&self.geoid_type(), target)),
            (Geoid::CensusTract(s, _, _), GeoidType::State) => Ok(Geoid::State(*s)),
            (Geoid::CensusTract(s, c, _), GeoidType::County) => Ok(Geoid::County(*s, *c)),
            (Geoid::CensusTract(_, _, _), GeoidType::CensusTract) => Ok(self.clone()),
            (Geoid::CensusTract(_, _, _), _) => Err(_err(&self.geoid_type(), target)),
            (Geoid::BlockGroup(s, _, _, _), GeoidType::State) => Ok(Geoid::State(*s)),
            (Geoid::BlockGroup(s, c, _, _), GeoidType::County) => Ok(Geoid::County(*s, *c)),
            (Geoid::BlockGroup(s, c, t, _), GeoidType::CensusTract) => {
                Ok(Geoid::CensusTract(*s, *c, *t))
            }
            (Geoid::BlockGroup(_, _, _, _), GeoidType::BlockGroup) => Ok(self.clone()),
            (Geoid::BlockGroup(_, _, _, _), _) => Err(_err(&self.geoid_type(), target)),
            (Geoid::Block(s, _, _, _), GeoidType::State) => Ok(Geoid::State(*s)),
            (Geoid::Block(s, c, _, _), GeoidType::County) => Ok(Geoid::County(*s, *c)),
            (Geoid::Block(s, c, t, _), GeoidType::CensusTract) => {
                Ok(Geoid::CensusTract(*s, *c, *t))
            }
            (Geoid::Block(s, c, t, b), GeoidType::BlockGroup) => {
                // special edge case of truncation, since we have no other operations for
                // converting between Block and Block Group.
                let block_str = &b.0[0..1];
                let bg = block_str
                    .parse::<u64>()
                    .map_err(|e| format!("cannot read first digit of block as integer: {e}"))?;
                let geoid = Geoid::BlockGroup(*s, *c, *t, fips::BlockGroup(bg));
                Ok(geoid)
            }
            (Geoid::Block(_, _, _, _), GeoidType::Block) => Ok(self.clone()),
            (Geoid::Block(_, _, _, _), _) => Err(_err(&self.geoid_type(), target)),
        }
    }

    /// predicate to filter by hierarchical geoshed.
    pub fn is_parent_of(&self, child: &Geoid) -> bool {
        match (self, child) {
            (Geoid::State(s1), Geoid::County(s2, _)) => s1 == s2,
            (Geoid::State(s1), Geoid::CountySubdivision(s2, _, _)) => s1 == s2,
            (Geoid::State(s1), Geoid::Place(s2, _)) => s1 == s2,
            (Geoid::State(s1), Geoid::CensusTract(s2, _, _)) => s1 == s2,
            (Geoid::State(s1), Geoid::BlockGroup(s2, _, _, _)) => s1 == s2,
            (Geoid::State(s1), Geoid::Block(s2, _, _, _)) => s1 == s2,
            (Geoid::County(s1, c1), Geoid::CountySubdivision(s2, c2, _)) => s1 == s2 && c1 == c2,
            (Geoid::County(s1, c1), Geoid::CensusTract(s2, c2, _)) => s1 == s2 && c1 == c2,
            (Geoid::County(s1, c1), Geoid::BlockGroup(s2, c2, _, _)) => s1 == s2 && c1 == c2,
            (Geoid::County(s1, c1), Geoid::Block(s2, c2, _, _)) => s1 == s2 && c1 == c2,
            (Geoid::CensusTract(s1, c1, t1), Geoid::BlockGroup(s2, c2, t2, _)) => {
                s1 == s2 && c1 == c2 && t1 == t2
            }
            (Geoid::CensusTract(s1, c1, t1), Geoid::Block(s2, c2, t2, _)) => {
                s1 == s2 && c1 == c2 && t1 == t2
            }
            (Geoid::BlockGroup(s1, c1, t1, bg1), Geoid::Block(s2, c2, t2, b2)) => {
                s1 == s2
                    && c1 == c2
                    && t1 == t2
                    && (b2.0.get(0..1).and_then(|s| s.parse::<u64>().ok()) == Some(bg1.0))
            }
            _ => false,
        }
    }

    /// manipulates this GEOID via truncation to transform it's GEOID type to that
    /// of it's parent.
    ///
    /// the base case is `None`, which is the parent of `State`, and signifies "no restriction"
    /// in census queries. for all other GeoidTypes, we simply remove the lowest area type.
    ///
    /// # Note
    ///
    /// Geoid::Block.to_parent() produces a CensusTract, not a BlockGroup, based on
    /// <https://www.census.gov/programs-surveys/geography/guidance/geo-identifiers.html>,
    /// which does not imply that all block groups are the first digit of all blocks.
    pub fn to_parent(&self) -> Option<Geoid> {
        match self {
            Geoid::State(_) => None,
            Geoid::County(s, _) => Some(Geoid::State(*s)),
            Geoid::CountySubdivision(s, c, _) => Some(Geoid::County(*s, *c)),
            Geoid::Place(s, _) => Some(Geoid::State(*s)),
            Geoid::CensusTract(s, c, _) => Some(Geoid::County(*s, *c)),
            Geoid::BlockGroup(s, c, t, _) => Some(Geoid::CensusTract(*s, *c, *t)),
            Geoid::Block(s, c, t, _) => Some(Geoid::CensusTract(*s, *c, *t)),
        }
    }

    pub fn to_state(&self) -> Geoid {
        match self {
            Geoid::State(_) => self.clone(),
            Geoid::County(st, _) => Geoid::State(*st),
            Geoid::CountySubdivision(st, _, _) => Geoid::State(*st),
            Geoid::Place(st, _) => Geoid::State(*st),
            Geoid::CensusTract(st, _, _) => Geoid::State(*st),
            Geoid::BlockGroup(st, _, _, _) => Geoid::State(*st),
            Geoid::Block(st, _, _, _) => Geoid::State(*st),
        }
    }

    pub fn to_state_abbreviation(&self) -> Result<String, String> {
        let state_fips = match self.to_state() {
            Geoid::State(s) => Ok(s),
            _ => Err(String::from("internal error")),
        }?;
        let state_code = StateCode::try_from(state_fips)?;
        let state_str = state_code.to_state_abbreviation();
        Ok(state_str)
    }

    pub fn to_county(&self) -> Result<Geoid, String> {
        match self {
            Geoid::State(_) => Err(String::from("state geoid does not contain a county geoid")),
            Geoid::County(st, ct) => Ok(Geoid::County(*st, *ct)),
            Geoid::CountySubdivision(st, ct, _) => Ok(Geoid::County(*st, *ct)),
            Geoid::Place(_, _) => Err(String::from("place geoid does not contain a county geoid")),
            Geoid::CensusTract(st, ct, _) => Ok(Geoid::County(*st, *ct)),
            Geoid::BlockGroup(st, ct, _, _) => Ok(Geoid::County(*st, *ct)),
            Geoid::Block(st, ct, _, _) => Ok(Geoid::County(*st, *ct)),
        }
    }

    pub fn to_census_tract(&self) -> Result<Geoid, String> {
        match self {
            Geoid::State(_) => Err(String::from(
                "state geoid does not contain a census tract geoid",
            )),
            Geoid::County(_, _) => Err(String::from(
                "county geoid does not contain a census tract geoid",
            )),
            Geoid::CountySubdivision(_, _, _) => Err(String::from(
                "county subdivision geoid does not contain a census tract geoid",
            )),
            Geoid::Place(_, _) => Err(String::from(
                "place geoid does not contain a census tract geoid",
            )),
            Geoid::CensusTract(st, ct, tr) => Ok(Geoid::CensusTract(*st, *ct, *tr)),
            Geoid::BlockGroup(st, ct, tr, _) => Ok(Geoid::CensusTract(*st, *ct, *tr)),
            Geoid::Block(st, ct, tr, _) => Ok(Geoid::CensusTract(*st, *ct, *tr)),
        }
    }
}

impl HasGeoidString for Geoid {
    /// Returns the canonical zero-padded numeric FIPS/GEOID string (e.g. `"08031"`),
    /// without the `<geoid_type>=` prefix produced by [`Display::fmt`] (e.g. `"county=08031"`).
    fn geoid_string(&self) -> String {
        match self {
            Geoid::State(st) => st.geoid_string(),
            Geoid::County(st, ct) => format!("{}{}", st.geoid_string(), ct.geoid_string()),
            Geoid::CountySubdivision(st, ct, cs) => format!(
                "{}{}{}",
                st.geoid_string(),
                ct.geoid_string(),
                cs.geoid_string()
            ),
            Geoid::Place(st, pl) => format!("{}{}", st.geoid_string(), pl.geoid_string()),
            Geoid::CensusTract(st, ct, tr) => format!(
                "{}{}{}",
                st.geoid_string(),
                ct.geoid_string(),
                tr.geoid_string()
            ),
            Geoid::BlockGroup(st, ct, tr, bg) => format!(
                "{}{}{}{}",
                st.geoid_string(),
                ct.geoid_string(),
                tr.geoid_string(),
                bg.geoid_string()
            ),
            Geoid::Block(st, ct, tr, bl) => format!(
                "{}{}{}{}",
                st.geoid_string(),
                ct.geoid_string(),
                tr.geoid_string(),
                bl.geoid_string()
            ),
        }
    }
}

impl Display for Geoid {
    /// Formats the GEOID with its hierarchy type prefix (e.g. `"county=08031"`).
    /// To obtain the bare zero-padded FIPS string (`"08031"`), use [`HasGeoidString::geoid_string`].
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.variant_name(), self.geoid_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_group_is_parent_of_block() {
        let bg = Geoid::BlockGroup(
            fips::State(8),
            fips::County(59),
            fips::CensusTract(100),
            fips::BlockGroup(2),
        );
        let matching_block = Geoid::Block(
            fips::State(8),
            fips::County(59),
            fips::CensusTract(100),
            fips::Block(String::from("2001")),
        );
        let non_matching_block = Geoid::Block(
            fips::State(8),
            fips::County(59),
            fips::CensusTract(100),
            fips::Block(String::from("1001")),
        );

        assert!(bg.is_parent_of(&matching_block));
        assert!(!bg.is_parent_of(&non_matching_block));
    }
}
