use super::TigerResource;
use bamcensus_core::model::identifier::{Geoid, GeoidType, HasGeoidString};
use std::{collections::HashSet, fmt::Display};

/// builds [`super::TigerResource`] instances for valid combinations of TIGER/Lines
/// years and GEOIDs.
///
/// support for a given year is based on understanding what the file naming
/// convention is for that year, how the data is organized, what the file
/// schema is.
pub enum TigerResourceBuilder {
    // /// <https://www2.census.gov/geo/tiger/TIGER2002/01_al/tgr01001.zip>
    // Tiger2002,
    // /// <https://www2.census.gov/geo/tiger/TIGER2003/01_AL/tgr01001.zip>
    // Tiger2003,
    // /// <https://www2.census.gov/geo/tiger/TIGER2008/01_ALABAMA/01001_Autauga/fe_2007_01001_tabblock00.zip>
    // Tiger2008,
    /// Use the 2010 format for the 2010 Tiger dataset
    ///
    /// # Examples
    ///  <https://www2.census.gov/geo/tiger/TIGER2010/TABBLOCK/2010/tl_2010_01001_tabblock10.zip>
    Tiger2010,
    /// Use the 2010 format for a given year
    ///
    /// # Examples
    ///  <https://www2.census.gov/geo/tiger/TIGER2011/TABBLOCK/tl_2011_01001_tabblock10.zip>
    Tiger2010Format { year: u64 },
    /// Use the 2020 format for a given year
    ///
    /// # Examples
    /// <https://www2.census.gov/geo/tiger/TIGER2020/TABBLOCK20/tl_2020_01_tabblock20.zip>
    Tiger2020Format { year: u64 },
}

impl Display for TigerResourceBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TigerResourceBuilder::Tiger2010 => write!(f, "TIGER2010"),
            TigerResourceBuilder::Tiger2010Format { year } => write!(f, "TIGER{year}"),
            TigerResourceBuilder::Tiger2020Format { year } => write!(f, "TIGER{year}"),
        }
    }
}

impl TigerResourceBuilder {
    pub const TIGER_BASE_URL: &'static str = "https://www2.census.gov/geo/tiger";

    pub fn new(year: u64) -> Result<TigerResourceBuilder, String> {
        match year {
            2010 => Ok(TigerResourceBuilder::Tiger2010),
            y if 2010 < y && y < 2020 => Ok(TigerResourceBuilder::Tiger2010Format { year }),
            y if 2020 <= y => Ok(TigerResourceBuilder::Tiger2020Format { year }),
            _ => Err(format!("unsupported TIGER year {year}")),
        }
    }

    /// batch operation that only returns the unique set of TigerUris required to cover
    /// the provided set of Geoids. this is the public API since we should only be
    /// downloading each file once. for details on implementation, see `create_resource`.
    pub fn create_resources(&self, geoids: &[&Geoid]) -> Result<Vec<TigerResource>, String> {
        let mut unique_uris: HashSet<TigerResource> = HashSet::new();
        for geoid in geoids {
            let uri = self.create_resource(geoid)?;
            unique_uris.insert(uri);
        }
        let uris = unique_uris.into_iter().collect::<Vec<_>>();
        Ok(uris)
    }

    /// creates a [`TigerResource`].
    /// in order to find the file matching this Geoid, we need to know what year
    /// and how that file is labeled. this matches against all years/geoid types
    /// to produce valid file URIs.
    ///
    /// # Example
    ///
    /// in this example, we construct the resource for county subdivision geometries in 2011,
    /// which are organized by state.
    ///
    /// ```rust
    /// use bamcensus_tiger::model::{TigerResourceBuilder, TigerResource};
    /// use bamcensus_core::model::identifier::{fips, Geoid, GeoidType};
    ///
    /// let builder = TigerResourceBuilder::Tiger2010Format { year: 2011 };
    /// let geoid = Geoid::CountySubdivision(
    ///     fips::State(48),
    ///     fips::County(13),
    ///     fips::CountySubdivision(90595)
    /// );
    /// let uri = builder.create_resource(&geoid).unwrap();
    /// let expected_uri = format!(
    ///     "{}/TIGER2011/COUSUB/tl_2011_48_cousub.zip",
    ///     TigerResourceBuilder::TIGER_BASE_URL
    /// );
    /// let expected_file_scope = GeoidType::State;
    /// let expected = TigerResource::new(
    ///     expected_uri,
    ///     GeoidType::CountySubdivision,
    ///     Some(expected_file_scope)
    /// );
    /// assert_eq!(uri, expected);
    /// ```
    pub fn create_resource(&self, geoid: &Geoid) -> Result<TigerResource, String> {
        let suffix: String = match (self, geoid) {
            //// ~~~~ 2010 ~~~~ ////
            // 2010 has two versions, one in 2000 format, one in 2010 format
            // so we have to add the "2010" directory to these
            (TigerResourceBuilder::Tiger2010, Geoid::State(state)) => {
                format!("STATE/2010/tl_2010_{}_state10.zip", state.geoid_string(),)
            }
            (TigerResourceBuilder::Tiger2010, Geoid::County(state, _)) => {
                format!("COUNTY/2010/tl_2010_{}_county10.zip", state.geoid_string(),)
            }
            (TigerResourceBuilder::Tiger2010, Geoid::CountySubdivision(state, county, _)) => {
                format!(
                    "COUSUB/2010/tl_2010_{}{}_cousub10.zip",
                    state.geoid_string(),
                    county.geoid_string()
                )
            }
            (TigerResourceBuilder::Tiger2010, Geoid::Place(state, _)) => {
                format!("PLACE/2010/tl_2010_{}_place10.zip", state.geoid_string(),)
            }
            (TigerResourceBuilder::Tiger2010, Geoid::CensusTract(state, county, _)) => format!(
                "TRACT/2010/tl_2010_{}{}_tract10.zip",
                state.geoid_string(),
                county.geoid_string()
            ),
            (TigerResourceBuilder::Tiger2010, Geoid::BlockGroup(state, county, _, _)) => format!(
                "BG/2010/tl_2010_{}{}_bg10.zip",
                state.geoid_string(),
                county.geoid_string()
            ),
            (TigerResourceBuilder::Tiger2010, Geoid::Block(state, county, _, _)) => format!(
                "TABBLOCK/2010/tl_2010_{}{}_tabblock10.zip",
                state.geoid_string(),
                county.geoid_string()
            ),
            //// ~~~~ 2011-2019 ~~~~ ////
            (TigerResourceBuilder::Tiger2010Format { year }, Geoid::State(_)) => {
                format!("STATE/tl_{year}_us_state.zip",)
            }
            (TigerResourceBuilder::Tiger2010Format { year }, Geoid::County(_, _)) => {
                format!("COUNTY/tl_{year}_us_county.zip")
            }
            (
                TigerResourceBuilder::Tiger2010Format { year },
                Geoid::CountySubdivision(state, _, _),
            ) => {
                format!("COUSUB/tl_{}_{}_cousub.zip", year, state.geoid_string())
            }
            (TigerResourceBuilder::Tiger2010Format { year }, Geoid::Place(state, _)) => {
                format!("PLACE/tl_{}_{}_place.zip", year, state.geoid_string(),)
            }
            (TigerResourceBuilder::Tiger2010Format { year }, Geoid::CensusTract(state, _, _)) => {
                format!("TRACT/tl_{}_{}_tract.zip", year, state.geoid_string())
            }

            (TigerResourceBuilder::Tiger2010Format { year }, Geoid::BlockGroup(state, _, _, _)) => {
                format!("BG/tl_{}_{}_bg.zip", year, state.geoid_string())
            }
            (TigerResourceBuilder::Tiger2010Format { year }, Geoid::Block(state, _, _, _)) => {
                format!(
                    "TABBLOCK/tl_{}_{}_tabblock10.zip",
                    year,
                    state.geoid_string()
                )
            }
            //// ~~~~ 2020-2029 ~~~~ ////
            (TigerResourceBuilder::Tiger2020Format { year }, Geoid::State(_)) => {
                format!("STATE/tl_{year}_us_state.zip",)
            }
            (TigerResourceBuilder::Tiger2020Format { year }, Geoid::County(_, _)) => {
                format!("COUNTY/tl_{year}_us_county.zip")
            }
            (
                TigerResourceBuilder::Tiger2020Format { year },
                Geoid::CountySubdivision(state, _, _),
            ) => {
                format!("COUSUB/tl_{}_{}_cousub.zip", year, state.geoid_string())
            }
            (TigerResourceBuilder::Tiger2020Format { year }, Geoid::Place(state, _)) => {
                format!("PLACE/tl_{}_{}_place.zip", year, state.geoid_string(),)
            }
            (TigerResourceBuilder::Tiger2020Format { year }, Geoid::CensusTract(state, _, _)) => {
                format!("TRACT/tl_{}_{}_tract.zip", year, state.geoid_string())
            }
            (TigerResourceBuilder::Tiger2020Format { year }, Geoid::BlockGroup(state, _, _, _)) => {
                format!("BG/tl_{}_{}_bg.zip", year, state.geoid_string())
            }
            (TigerResourceBuilder::Tiger2020Format { year }, Geoid::Block(state, _, _, _)) => {
                format!(
                    "TABBLOCK20/tl_{}_{}_tabblock20.zip",
                    year,
                    state.geoid_string()
                )
            }
        };

        let file_scope = match (self, geoid) {
            (TigerResourceBuilder::Tiger2010, Geoid::State(_)) => Some(GeoidType::State),
            (TigerResourceBuilder::Tiger2010, Geoid::County(_, _)) => Some(GeoidType::State),
            (TigerResourceBuilder::Tiger2010, Geoid::CountySubdivision(_, _, _)) => {
                Some(GeoidType::County)
            }
            (TigerResourceBuilder::Tiger2010, Geoid::Place(_, _)) => Some(GeoidType::State),
            (TigerResourceBuilder::Tiger2010, Geoid::CensusTract(_, _, _)) => {
                Some(GeoidType::County)
            }
            (TigerResourceBuilder::Tiger2010, Geoid::BlockGroup(_, _, _, _)) => {
                Some(GeoidType::County)
            }
            (TigerResourceBuilder::Tiger2010, Geoid::Block(_, _, _, _)) => Some(GeoidType::County),
            (TigerResourceBuilder::Tiger2010Format { year: _ }, Geoid::State(_)) => None,
            (TigerResourceBuilder::Tiger2010Format { year: _ }, Geoid::County(_, _)) => None,
            (
                TigerResourceBuilder::Tiger2010Format { year: _ },
                Geoid::CountySubdivision(_, _, _),
            ) => Some(GeoidType::State),
            (TigerResourceBuilder::Tiger2010Format { year: _ }, Geoid::Place(_, _)) => {
                Some(GeoidType::State)
            }
            (TigerResourceBuilder::Tiger2010Format { year: _ }, Geoid::CensusTract(_, _, _)) => {
                Some(GeoidType::State)
            }
            (TigerResourceBuilder::Tiger2010Format { year: _ }, Geoid::BlockGroup(_, _, _, _)) => {
                Some(GeoidType::State)
            }
            (TigerResourceBuilder::Tiger2010Format { year: _ }, Geoid::Block(_, _, _, _)) => {
                Some(GeoidType::State)
            }
            (TigerResourceBuilder::Tiger2020Format { year: _ }, Geoid::State(_)) => None,
            (TigerResourceBuilder::Tiger2020Format { year: _ }, Geoid::County(_, _)) => None,
            (
                TigerResourceBuilder::Tiger2020Format { year: _ },
                Geoid::CountySubdivision(_, _, _),
            ) => Some(GeoidType::State),
            (TigerResourceBuilder::Tiger2020Format { year: _ }, Geoid::Place(_, _)) => {
                Some(GeoidType::State)
            }
            (TigerResourceBuilder::Tiger2020Format { year: _ }, Geoid::CensusTract(_, _, _)) => {
                Some(GeoidType::State)
            }
            (TigerResourceBuilder::Tiger2020Format { year: _ }, Geoid::BlockGroup(_, _, _, _)) => {
                Some(GeoidType::State)
            }
            (TigerResourceBuilder::Tiger2020Format { year: _ }, Geoid::Block(_, _, _, _)) => {
                Some(GeoidType::State)
            }
        };

        let prefix = self.base_url();
        let uri = format!("{prefix}/{suffix}");
        let geoid_type = geoid.geoid_type();
        // let geoid_column_name = self.geoid_shapefile_colname(&suffix);
        let tiger_uri = TigerResource::new(uri, geoid_type, file_scope); //, geoid_column_name);
        Ok(tiger_uri)
    }

    // pub fn geoid_shapefile_colname(&self, filename: &str) -> String {
    //     // handle the GEOID column naming conventions that differ under
    //     // edge cases, such as TABBLOCK10 in 2010 + TABBLOCK20 in 2020
    //     // matching off of "n.zip" as a quick and easy pattern match
    //     if self.get_year() == 2010 && filename.ends_with("10.zip") {
    //         String::from("GEOID10")
    //     } else if self.get_year() == 2020 && filename.ends_with("20.zip") {
    //         String::from("GEOID20")
    //     } else {
    //         String::from("GEOID")
    //     }
    // }

    /// gets the year for this builder
    fn get_year(&self) -> u64 {
        match self {
            TigerResourceBuilder::Tiger2010 => 2010,
            TigerResourceBuilder::Tiger2010Format { year } => *year,
            TigerResourceBuilder::Tiger2020Format { year } => *year,
        }
    }

    /// creates a URL to a TIGER file location.
    fn base_url(&self) -> String {
        let year = self.get_year();
        format!("{}/TIGER{}", TigerResourceBuilder::TIGER_BASE_URL, year)
    }
}
