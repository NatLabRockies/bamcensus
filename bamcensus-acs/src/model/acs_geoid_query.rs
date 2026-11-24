use bamcensus_core::model::identifier::{fips, Geoid, GeoidType, HasGeoidString};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

use super::DeserializeGeoidFn;

/// enumeration representing the scopes of various ACS queries.
///
/// when running an ACS query at a given GEOID hierarchical level, there are a set
/// of required (aka, not `Option`al) components which can be coupled with `Option`al
/// (wildcard) components to construct a query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AcsGeoidQuery {
    State(Option<fips::State>),
    County(Option<fips::State>, Option<fips::County>),
    CountySubdivision(
        fips::State,
        Option<fips::County>,
        Option<fips::CountySubdivision>,
    ),
    Place(Option<fips::State>, Option<fips::Place>),
    CensusTract(fips::State, Option<fips::County>, Option<fips::CensusTract>),
    BlockGroup(
        fips::State,
        Option<fips::County>,
        Option<fips::CensusTract>,
        Option<fips::BlockGroup>,
    ),
}

impl AcsGeoidQuery {
    /// # Examples
    ///
    /// when no wildcard is provided, the query is constructed for an exact location, which
    /// should return a single result.
    /// ```rust
    /// use bamcensus_core::model::identifier::{fips, Geoid, GeoidType};
    /// use bamcensus_acs::model::AcsGeoidQuery;;
    ///
    /// let geoid = Geoid::County(fips::State(8), fips::County(1));
    /// let query = AcsGeoidQuery::new(Some(geoid), None).unwrap();
    /// let key = query.to_query_key();
    /// assert_eq!(key, String::from("&for=county:001&in=state:08"));
    /// ```
    ///
    /// some combinations simply append a wildcard one level below the geoid, for example,
    /// tacking a county=* on top of state=08.
    /// ```rust
    /// use bamcensus_core::model::identifier::{fips, Geoid, GeoidType};
    /// use bamcensus_acs::model::AcsGeoidQuery;;
    ///
    /// let geoid = Geoid::State(fips::State(8));
    /// let wildcard = GeoidType::County;
    /// let query = AcsGeoidQuery::new(Some(geoid), Some(wildcard)).unwrap();
    /// let key = query.to_query_key();
    /// assert_eq!(key, String::from("&for=county:*&in=state:08"));
    /// ```
    ///
    /// some interpolate the wildcard into a geoid in ways that do not require being
    /// reported in the query, such as adding a wildcard to a tract query. in this case,
    /// the query simply drops the county wildcard, as it is implied.
    /// ```rust
    /// use bamcensus_core::model::identifier::{fips, Geoid, GeoidType};
    /// use bamcensus_acs::model::AcsGeoidQuery;;
    ///
    /// let geoid = Geoid::CensusTract(fips::State(8), fips::County(1), fips::CensusTract(1));
    /// let wildcard = GeoidType::County;
    /// let query = AcsGeoidQuery::new(Some(geoid), Some(wildcard)).unwrap();
    /// let key = query.to_query_key();
    /// assert_eq!(key, String::from("&for=tract:000001&in=state:08"));
    /// ```
    ///
    /// # Returns
    ///
    /// URL query string for calls to the US Census ACS API "for" section, which set the
    /// spatial scope and granularity of the query result.
    pub fn new(geoid: Option<Geoid>, wildcard: Option<GeoidType>) -> Result<AcsGeoidQuery, String> {
        use Geoid as G;
        use GeoidType as GT;

        match (geoid, wildcard) {
            // ~~ errors ~~
            // - invalid combinations of geoid/wildcard values
            (None, None) => Err(String::from(
                "cannot create query without at least a geoid or wildcard",
            )),
            (None, Some(GT::CountySubdivision)) => Err(String::from(
                "cannot create county subdivision query without State Geoid",
            )),
            (None, Some(GT::CensusTract)) => Err(String::from(
                "cannot create census tract query without State Geoid",
            )),
            (None, Some(GT::BlockGroup)) => Err(String::from(
                "cannot create block group query without State + County Geoids",
            )),
            (_, Some(GT::Block)) => Err(String::from("acs does not support block-level queries")),
            (Some(G::Block(_, _, _, _)), _) => {
                Err(String::from("acs does not support block-level queries"))
            }

            (Some(Geoid::State(_)), Some(GT::BlockGroup)) => Err(String::from(
                "cannot create block group query without County Geoid",
            )),
            (Some(Geoid::County(_, _)), Some(GT::Place)) => Err(String::from(
                "cannot append a 'Place' wildcard to a County Geoid",
            )),
            (Some(G::CountySubdivision(_, _, _)), Some(GT::Place)) => Err(String::from(
                "cannot append a 'Place' wildcard to a CountySubdivision Geoid",
            )),
            (Some(G::CountySubdivision(_, _, _)), Some(GT::CensusTract)) => Err(String::from(
                "cannot append a 'CensusTract' wildcard to a CountySubdivision Geoid",
            )),
            (Some(G::CountySubdivision(_, _, _)), Some(GT::BlockGroup)) => Err(String::from(
                "cannot append a 'BlockGroup' wildcard to a CountySubdivision Geoid",
            )),
            (Some(Geoid::Place(_, _)), Some(GT::County)) => Err(String::from(
                "cannot append a 'County' wildcard to a Place Geoid",
            )),
            (Some(Geoid::Place(_, _)), Some(GT::CountySubdivision)) => Err(String::from(
                "cannot append a 'CountySubdivision' wildcard to a Place Geoid",
            )),
            (Some(Geoid::Place(_, _)), Some(GT::CensusTract)) => Err(String::from(
                "cannot append a 'CensusTract' wildcard to a Place Geoid",
            )),
            (Some(Geoid::Place(_, _)), Some(GT::BlockGroup)) => Err(String::from(
                "cannot append a 'BlockGroup' wildcard to a Place Geoid",
            )),
            (Some(Geoid::CensusTract(_, _, _)), Some(GT::State)) => Err(String::from(
                "cannot append a 'State' wildcard to a CensusTract Geoid",
            )),
            (Some(Geoid::CensusTract(_, _, _)), Some(GT::CountySubdivision)) => Err(String::from(
                "cannot append a 'CountySubdivision' wildcard to a CensusTract Geoid",
            )),
            (Some(Geoid::CensusTract(_, _, _)), Some(GT::Place)) => Err(String::from(
                "cannot append a 'Place' wildcard to a CensusTract Geoid",
            )),
            (Some(Geoid::BlockGroup(_, _, _, _)), Some(GT::State)) => Err(String::from(
                "cannot append a 'State' wildcard to a BlockGroup Geoid",
            )),
            (Some(Geoid::BlockGroup(_, _, _, _)), Some(GT::CountySubdivision)) => Err(
                String::from("cannot append a 'CountySubdivision' wildcard to a BlockGroup Geoid"),
            ),
            (Some(Geoid::BlockGroup(_, _, _, _)), Some(GT::Place)) => Err(String::from(
                "cannot append a 'Place' wildcard to a BlockGroup Geoid",
            )),

            // ~~ wildcard-only queries for different GEOID levels ~~
            (None, Some(GT::State)) => Ok(AcsGeoidQuery::State(None)),
            (None, Some(GT::County)) => Ok(AcsGeoidQuery::County(None, None)),
            (None, Some(GT::Place)) => Ok(AcsGeoidQuery::Place(None, None)),

            // ~~ queries for wildcards inserted into specific geoids ~~
            // - STATE -
            (Some(Geoid::State(_)), Some(GT::State)) => Ok(AcsGeoidQuery::State(None)),
            (Some(Geoid::State(s)), Some(GT::County)) => Ok(AcsGeoidQuery::County(Some(s), None)),
            (Some(Geoid::State(s)), Some(GT::CountySubdivision)) => {
                Ok(AcsGeoidQuery::CountySubdivision(s, None, None))
            }
            (Some(Geoid::State(s)), Some(GT::Place)) => Ok(AcsGeoidQuery::Place(Some(s), None)),
            (Some(Geoid::State(s)), Some(GT::CensusTract)) => {
                Ok(AcsGeoidQuery::CensusTract(s, None, None))
            }

            // - COUNTY -
            (Some(Geoid::County(_, c)), Some(GT::State)) => {
                Ok(AcsGeoidQuery::County(None, Some(c)))
            }
            (Some(Geoid::County(s, _)), Some(GT::County)) => {
                Ok(AcsGeoidQuery::County(Some(s), None))
            }
            (Some(Geoid::County(s, c)), Some(GT::CountySubdivision)) => {
                Ok(AcsGeoidQuery::CountySubdivision(s, Some(c), None))
            }
            (Some(Geoid::County(s, c)), Some(GT::CensusTract)) => {
                Ok(AcsGeoidQuery::CensusTract(s, Some(c), None))
            }
            (Some(Geoid::County(s, c)), Some(GT::BlockGroup)) => {
                Ok(AcsGeoidQuery::BlockGroup(s, Some(c), None, None))
            }

            // - COUNTY SUBDIVISION -
            (Some(G::CountySubdivision(st, ct, cs)), Some(GT::State)) => {
                Ok(AcsGeoidQuery::CountySubdivision(st, Some(ct), Some(cs)))
            }
            (Some(G::CountySubdivision(s, _, cs)), Some(GT::County)) => {
                Ok(AcsGeoidQuery::CountySubdivision(s, None, Some(cs)))
            }
            (Some(G::CountySubdivision(s, ct, _)), Some(GT::CountySubdivision)) => {
                Ok(AcsGeoidQuery::CountySubdivision(s, Some(ct), None))
            }

            // - PLACE -
            (Some(Geoid::Place(_, p)), Some(GT::State)) => Ok(AcsGeoidQuery::Place(None, Some(p))),
            (Some(Geoid::Place(s, _)), Some(GT::Place)) => Ok(AcsGeoidQuery::Place(Some(s), None)),

            // - CENSUS TRACT -
            (Some(Geoid::CensusTract(s, _, t)), Some(GT::County)) => {
                Ok(AcsGeoidQuery::CensusTract(s, None, Some(t)))
            }
            (Some(Geoid::CensusTract(s, c, _)), Some(GT::CensusTract)) => {
                Ok(AcsGeoidQuery::CensusTract(s, Some(c), None))
            }
            (Some(Geoid::CensusTract(s, c, t)), Some(GT::BlockGroup)) => {
                Ok(AcsGeoidQuery::BlockGroup(s, Some(c), Some(t), None))
            }

            // - BLOCK GROUP -
            (Some(Geoid::BlockGroup(s, _, t, b)), Some(GT::County)) => {
                Ok(AcsGeoidQuery::BlockGroup(s, None, Some(t), Some(b)))
            }
            (Some(Geoid::BlockGroup(s, c, _, b)), Some(GT::CensusTract)) => {
                Ok(AcsGeoidQuery::BlockGroup(s, Some(c), None, Some(b)))
            }
            (Some(Geoid::BlockGroup(s, c, t, _)), Some(GT::BlockGroup)) => {
                Ok(AcsGeoidQuery::BlockGroup(s, Some(c), Some(t), None))
            }

            // ~~ queries for specific geoids (no wildcards) ~~
            (Some(Geoid::State(s)), None) => Ok(AcsGeoidQuery::State(Some(s))),
            (Some(Geoid::County(s, c)), None) => Ok(AcsGeoidQuery::County(Some(s), Some(c))),
            (Some(Geoid::CountySubdivision(s, ct, cs)), None) => {
                Ok(AcsGeoidQuery::CountySubdivision(s, Some(ct), Some(cs)))
            }
            (Some(Geoid::Place(s, p)), None) => Ok(AcsGeoidQuery::Place(Some(s), Some(p))),
            (Some(Geoid::CensusTract(s, c, t)), None) => {
                Ok(AcsGeoidQuery::CensusTract(s, Some(c), Some(t)))
            }
            (Some(Geoid::BlockGroup(s, c, t, b)), None) => {
                Ok(AcsGeoidQuery::BlockGroup(s, Some(c), Some(t), Some(b)))
            }
        }
    }

    /// a query key for a unique data row in the census API. depending on the AcsGeoidQuery
    /// and the presence/absence of FIPS values, wildcards ("*") will be inserted at any level.
    pub fn to_query_key(&self) -> String {
        use AcsGeoidQuery as G;
        match self {
            G::State(state) => match state {
                None => String::from("&for=state:*"),
                Some(s) => format!("&for=state:{}", s.geoid_string()),
            },
            G::County(state, county) => match (state, county) {
                (None, None) => String::from("&for=county:*"),
                (None, Some(c)) => format!("&for=county:{}", c.geoid_string()),
                (Some(s), None) => format!("&for=county:*&in=state:{}", s.geoid_string()),
                (Some(s), Some(c)) => format!(
                    "&for=county:{}&in=state:{}",
                    c.geoid_string(),
                    s.geoid_string()
                ),
            },
            G::CountySubdivision(state, county, cousub) => match (county, cousub) {
                (None, None) => format!(
                    "&for=county%20subdivision:*&in=state:{}&in=county:*",
                    state.geoid_string(),
                ),
                (None, Some(cs)) => format!(
                    "&for=county%20subdivision:{}&in=state:{}&in=county:*",
                    cs.geoid_string(),
                    state.geoid_string(),
                ),
                (Some(co), None) => format!(
                    "&for=county%20subdivision:*&in=state:{}&in=county:{}",
                    state.geoid_string(),
                    co.geoid_string(),
                ),
                (Some(co), Some(cs)) => format!(
                    "&for=county%20subdivision:{}&in=state:{}&in=county:{}",
                    cs.geoid_string(),
                    state.geoid_string(),
                    co.geoid_string()
                ),
            },
            G::Place(state, place) => match (state, place) {
                (None, None) => String::from("&for=place:*"),
                (None, Some(pl)) => format!("&for=place:{}&in=state:*", pl.geoid_string()),
                (Some(st), None) => format!("&for=place:*&in=state:{}", st.geoid_string()),
                (Some(st), Some(pl)) => format!(
                    "&for=place:{}&in=state:{}",
                    pl.geoid_string(),
                    st.geoid_string()
                ),
            },
            G::CensusTract(state, county, tract) => match (county, tract) {
                (None, None) => format!("&for=tract:*&in=state:{}", state.geoid_string()),
                (None, Some(tr)) => format!(
                    "&for=tract:{}&in=state:{}",
                    tr.geoid_string(),
                    state.geoid_string()
                ),
                (Some(co), None) => format!(
                    "&for=tract:*&in=state:{}&in=county:{}",
                    state.geoid_string(),
                    co.geoid_string()
                ),
                (Some(co), Some(tr)) => format!(
                    "&for=tract:{}&in=state:{}&in=county:{}",
                    tr.geoid_string(),
                    state.geoid_string(),
                    co.geoid_string()
                ),
            },
            G::BlockGroup(state, county, tract, block_group) => {
                match (county, tract, block_group) {
                    (None, None, None) => format!(
                        "&for=block%20group:*&in=state:{}&in=county:*&in=tract:*",
                        state.geoid_string()
                    ),
                    (None, None, Some(b)) => format!(
                        "&for=block%20group:{}&in=state:{}&in=county:*&in=tract:*",
                        b.geoid_string(),
                        state.geoid_string()
                    ),
                    (None, Some(t), None) => format!(
                        "&for=block%20group:*&in=state:{}&in=county:*&in=tract:{}",
                        state.geoid_string(),
                        t.geoid_string(),
                    ),
                    (None, Some(t), Some(b)) => format!(
                        "&for=block%20group:{}&in=state:{}&in=county:*&in=tract:{}",
                        b.geoid_string(),
                        state.geoid_string(),
                        t.geoid_string(),
                    ),
                    (Some(c), None, None) => format!(
                        "&for=block%20group:*&in=state:{}&in=county:{}&in=tract:*",
                        state.geoid_string(),
                        c.geoid_string()
                    ),
                    (Some(c), None, Some(b)) => format!(
                        "&for=block%20group:{}&in=state:{}&in=county:{}&in=tract:*",
                        b.geoid_string(),
                        state.geoid_string(),
                        c.geoid_string(),
                    ),
                    (Some(c), Some(t), None) => format!(
                        "&for=block%20group:*&in=state:{}&in=county:{}&in=tract:{}",
                        state.geoid_string(),
                        c.geoid_string(),
                        t.geoid_string(),
                    ),
                    (Some(c), Some(t), Some(b)) => format!(
                        "&for=block%20group:{}&in=state:{}&in=county:{}&in=tract:{}",
                        b.geoid_string(),
                        state.geoid_string(),
                        c.geoid_string(),
                        t.geoid_string()
                    ),
                }
            }
        }
    }

    pub fn response_geoid_type(&self) -> GeoidType {
        use AcsGeoidQuery as G;
        match self {
            G::State(_) => GeoidType::State,
            G::County(_, _) => GeoidType::County,
            G::CountySubdivision(_, _, _) => GeoidType::CountySubdivision,
            G::Place(_, _) => GeoidType::Place,
            G::CensusTract(_, _, _) => GeoidType::CensusTract,
            G::BlockGroup(_, _, _, _) => GeoidType::BlockGroup,
        }
    }

    pub fn response_column_names(&self) -> Vec<String> {
        use AcsGeoidQuery as G;
        match self {
            G::State(_) => vec![String::from("state")],
            G::County(_, _) => vec![String::from("state"), String::from("county")],
            G::CountySubdivision(_, _, _) => vec![
                String::from("state"),
                String::from("county"),
                String::from("county subdivision"),
            ],
            G::Place(_, _) => vec![String::from("state"), String::from("place")],
            G::CensusTract(_, _, _) => vec![
                String::from("state"),
                String::from("county"),
                String::from("tract"),
            ],
            G::BlockGroup(_, _, _, _) => vec![
                String::from("state"),
                String::from("county"),
                String::from("tract"),
                String::from("block group"),
            ],
        }
    }

    pub fn response_column_count(&self) -> usize {
        match self {
            AcsGeoidQuery::State(_) => 1,
            AcsGeoidQuery::County(_, _) => 2,
            AcsGeoidQuery::CountySubdivision(_, _, _) => 3,
            AcsGeoidQuery::Place(_, _) => 2,
            AcsGeoidQuery::CensusTract(_, _, _) => 3,
            AcsGeoidQuery::BlockGroup(_, _, _, _) => 4,
        }
    }

    pub fn get_geoid_type(&self) -> GeoidType {
        match self {
            AcsGeoidQuery::State(_) => GeoidType::State,
            AcsGeoidQuery::County(_, _) => GeoidType::County,
            AcsGeoidQuery::CountySubdivision(_, _, _) => GeoidType::CountySubdivision,
            AcsGeoidQuery::Place(_, _) => GeoidType::Place,
            AcsGeoidQuery::CensusTract(_, _, _) => GeoidType::CensusTract,
            AcsGeoidQuery::BlockGroup(_, _, _, _) => GeoidType::BlockGroup,
        }
    }

    /// builds a function that unpacks the ACS query values representing a geoid.
    /// these return as values in an array of different lengths, depending on the scope of
    /// the original query.
    pub fn build_deserialize_geoid_fn(&self) -> DeserializeGeoidFn {
        let geoid_type = self.get_geoid_type();
        let f: DeserializeGeoidFn = Rc::new(move |vals| {
            let strings = as_strings(&vals)?;
            geoid_type.geoid_from_slice_of_strings(&strings)
        });
        f
    }
}

/// helper function to convert a vec of JSON values to their expected String values.
fn as_strings(arr: &[serde_json::Value]) -> Result<Vec<String>, String> {
    arr.iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| format!("raw geoid value should be string, found {v}"))
                .map(String::from)
        })
        .collect::<Result<Vec<_>, String>>()
}
