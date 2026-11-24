use crate::model::{AcsGeoidQuery, AcsType};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// parameters for running an ACS call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AcsApiQueryParams {
    /// optional override URL for ACS query. otherwise defaults to [`super::constants::BASE_URL`]
    pub base_url: Option<String>,
    /// ACS dataset yera
    pub year: u64,
    /// one or five-year ACS cadence
    pub acs_type: AcsType,
    /// list of fields to retrieve
    pub get_query: Vec<String>,
    /// geographic scope of request
    pub for_query: AcsGeoidQuery,
    /// optional API token in case of rate limiting issues
    pub api_token: Option<String>,
}

impl AcsApiQueryParams {
    pub fn new(
        base_url: Option<String>,
        year: u64,
        acs_type: AcsType,
        get_query: Vec<String>,
        for_query: AcsGeoidQuery,
        api_token: Option<String>,
    ) -> AcsApiQueryParams {
        AcsApiQueryParams {
            base_url,
            year,
            acs_type,
            get_query,
            for_query,
            api_token,
        }
    }

    pub fn acs_dataset_url(&self) -> String {
        let base = self
            .base_url
            .clone()
            .unwrap_or(String::from(super::constants::BASE_URL));
        let type_s = self.acs_type.to_directory_name();
        format!("{}/{}/acs/{}", base, self.year, type_s)
    }

    /// builds an ACS REST query URL from application parameters.
    ///
    /// # Examples
    ///
    /// Example 1. Get 2022 Five-Year ACS state-level population estimates for all states, returning
    /// their state name, population value, and state FIPS code.
    ///
    /// ```rust
    /// use bamcensus_core::model::identifier::{fips, GeoidType};
    /// use bamcensus_acs::model::{
    ///     AcsType,
    ///     AcsGeoidQuery,
    ///     AcsApiQueryParams
    /// };
    ///
    /// let base_url = String::from("https://api.census.gov/data");
    /// let acs_year: u64 = 2022;
    /// let acs_type: AcsType = AcsType::FiveYear;
    /// let queries = vec![String::from("NAME"), String::from("B01001_001E")];
    /// let acs_geoid_query: AcsGeoidQuery = AcsGeoidQuery::new(None, Some(GeoidType::State)).unwrap();
    /// let api_query_params = AcsApiQueryParams::new(Some(base_url), acs_year, acs_type, queries, acs_geoid_query, None);
    /// let api_url = api_query_params.build_url().unwrap();
    /// assert_eq!(api_url, String::from("https://api.census.gov/data/2022/acs/acs5?get=NAME,B01001_001E&for=state:*"))
    /// ```
    /// Example 2. Get 2022 Five-Year ACS state-level population estimates for all counties in Colorado, returning
    /// their state name, population value, and state FIPS code.
    ///
    /// ```rust
    /// use bamcensus_core::model::identifier::{fips, GeoidType, Geoid};
    /// use bamcensus_acs::model::{
    ///     AcsType,
    ///     AcsGeoidQuery,
    ///     AcsApiQueryParams
    /// };
    ///
    /// let base_url = String::from("https://api.census.gov/data");
    /// let acs_year: u64 = 2022;
    /// let acs_type: AcsType = AcsType::FiveYear;
    /// let queries = vec![String::from("NAME"), String::from("B01001_001E")];
    /// let acs_geoid_query: AcsGeoidQuery = AcsGeoidQuery::new(Some(Geoid::State(fips::State(08))), Some(GeoidType::County)).unwrap();
    /// let api_query_params = AcsApiQueryParams::new(Some(base_url), acs_year, acs_type, queries, acs_geoid_query, None);
    /// let api_url = api_query_params.build_url().unwrap();
    /// assert_eq!(api_url, String::from("https://api.census.gov/data/2022/acs/acs5?get=NAME,B01001_001E&for=county:*&in=state:08"))
    /// ```
    pub fn build_url(&self) -> Result<String, String> {
        let dataset_url = self.acs_dataset_url();
        let get_query = self.get_query.iter().join(",");
        let for_query = self.for_query.to_query_key();
        let token_query = match &self.api_token {
            Some(k) => format!("&key={k}"),
            None => String::from(""),
        };
        let query = format!("{dataset_url}?get={get_query}{for_query}{token_query}",);
        Ok(query)
    }

    /// in order to deconstruct an API response, we need the list of
    /// column names in the order that they would appear in the array-
    /// shaped ACS response object.
    pub fn column_names(&self) -> Vec<String> {
        let mut cols = self
            .get_query
            .iter()
            // .map(|s| String::from(s))
            .cloned()
            .collect_vec();
        cols.extend(self.for_query.response_column_names());
        cols
    }

    pub fn output_filename(&self) -> String {
        let get_query = self.get_query.join("&");
        let for_query = self.for_query.to_query_key();

        format!(
            "{}-{}-{}-{}.csv",
            self.acs_type, self.year, get_query, for_query
        )
    }
}
