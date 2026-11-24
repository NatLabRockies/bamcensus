//! Reusable, serde-ready configuration structs for ACS and LODES + TIGER workflows.
//!
//! These structs can be embedded directly or used with `#[serde(flatten)]` in downstream
//! application configurations (such as `bambam` and `bambam-congestion`).

use crate::app::acs_tiger::{self, AcsRegionQueryParams, AcsTigerResponse};
use crate::app::lodes_tiger::{self, LodesTigerResponse};
use bamcensus_acs::model::constants::{DEFAULT_ACS_POPULATION_VARIABLE, DEFAULT_ACS_YEAR};
use bamcensus_acs::model::AcsType;
use bamcensus_core::model::identifier::GeoidType;
use bamcensus_core::model::spatial::RegionFilter;
use bamcensus_core::ops::http::ClientWithMiddleware;
use bamcensus_lehd::model::{
    FallbackPolicy, LodesDataset, LodesEdition, LodesJobType, WacSegment, WorkplaceSegment,
    LATEST_YEAR,
};
use serde::{Deserialize, Serialize};

/// Default ACS survey vintage year (`2025`).
pub fn default_acs_year() -> u64 {
    DEFAULT_ACS_YEAR
}

/// Default ACS geospatial resolution ([`GeoidType::BlockGroup`]).
pub fn default_acs_resolution() -> GeoidType {
    GeoidType::BlockGroup
}

/// Default ACS variable categories (`["B01001_001E"]` — total population).
pub fn default_acs_categories() -> Vec<String> {
    vec![String::from(DEFAULT_ACS_POPULATION_VARIABLE)]
}

/// Default LODES data granularity (`Some(GeoidType::BlockGroup)`).
pub fn default_lodes_granularity() -> Option<GeoidType> {
    Some(GeoidType::BlockGroup)
}

/// Default LODES dataset year ([`LATEST_YEAR`], `2024`).
pub fn default_lodes_year() -> u64 {
    LATEST_YEAR
}

/// Default LODES WAC segments (`[WacSegment::C000]` — total jobs).
pub fn default_wac_segments() -> Vec<WacSegment> {
    vec![WacSegment::C000]
}

/// Configuration for US Census ACS + TIGER/Line queries.
///
/// Supports `#[derive(Default)]`, partial JSON/TOML/YAML deserialization via `#[serde(default)]`,
/// and `#[serde(flatten)]` in downstream configuration structs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AcsTigerConfig {
    /// One-year or five-year ACS estimates (defaults to [`AcsType::FiveYear`]).
    #[serde(default)]
    pub acs_type: AcsType,
    /// ACS and TIGER dataset vintage year (defaults to `2025`).
    #[serde(default = "default_acs_year")]
    pub acs_year: u64,
    /// Geospatial resolution for ACS queries (defaults to [`GeoidType::BlockGroup`]).
    #[serde(default = "default_acs_resolution")]
    pub acs_resolution: GeoidType,
    /// ACS variable identifiers to query (defaults to `["B01001_001E"]`).
    #[serde(default = "default_acs_categories")]
    pub acs_categories: Vec<String>,
    /// Optional Census API token. If `None`, [`Self::resolve_api_token`] checks `CENSUS_API_TOKEN`.
    #[serde(default)]
    pub api_token: Option<String>,
}

impl Default for AcsTigerConfig {
    fn default() -> Self {
        Self {
            acs_type: AcsType::default(),
            acs_year: default_acs_year(),
            acs_resolution: default_acs_resolution(),
            acs_categories: default_acs_categories(),
            api_token: None,
        }
    }
}

impl AcsTigerConfig {
    /// Resolves the Census API token from [`Self::api_token`] or the `CENSUS_API_TOKEN`
    /// environment variable if not explicitly set in the configuration.
    pub fn resolve_api_token(&self) -> Option<String> {
        self.api_token
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::env::var("CENSUS_API_TOKEN")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
    }

    /// Converts this configuration and a target [`RegionFilter`] into [`AcsRegionQueryParams`].
    pub fn to_region_query_params(&self, region: RegionFilter) -> AcsRegionQueryParams {
        AcsRegionQueryParams {
            base_url: None,
            year: self.acs_year,
            acs_type: self.acs_type,
            acs_get_query: self.acs_categories.clone(),
            region,
            output_resolution: Some(self.acs_resolution),
            acs_token: self.resolve_api_token(),
        }
    }

    /// Runs the ACS + TIGER workflow for the given [`RegionFilter`].
    pub async fn run_region(&self, region: RegionFilter) -> Result<AcsTigerResponse, String> {
        let params = self.to_region_query_params(region);
        acs_tiger::run_region(&params).await
    }

    /// Runs the ACS + TIGER workflow for the given [`RegionFilter`] using the supplied HTTP client.
    pub async fn run_region_with_client(
        &self,
        client: &ClientWithMiddleware,
        region: RegionFilter,
    ) -> Result<AcsTigerResponse, String> {
        let params = self.to_region_query_params(region);
        acs_tiger::run_region_with_client(client, &params).await
    }
}

/// Configuration for US Census LEHD LODES Workplace Area Characteristics (WAC) + TIGER/Line queries.
///
/// Supports `#[derive(Default)]`, partial JSON/TOML/YAML deserialization via `#[serde(default)]`,
/// and `#[serde(flatten)]` in downstream configuration structs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LodesWacTigerConfig {
    /// Study region filter (explicit GEOIDs or polygon). If `None`, callers may supply a region
    /// at execution time or default to [`RegionFilter::national`].
    #[serde(default)]
    pub study_region: Option<RegionFilter>,
    /// Output geospatial aggregation resolution (defaults to `Some(GeoidType::BlockGroup)`).
    #[serde(default = "default_lodes_granularity")]
    pub data_granularity: Option<GeoidType>,
    /// LODES edition / geography vintage (defaults to [`LodesEdition::Lodes8`]).
    #[serde(default)]
    pub edition: LodesEdition,
    /// LODES job type filter (defaults to [`LodesJobType::JT00`]).
    #[serde(default)]
    pub job_type: LodesJobType,
    /// LODES workplace workforce segment (defaults to [`WorkplaceSegment::S000`]).
    #[serde(default)]
    pub segment: WorkplaceSegment,
    /// LODES dataset year (defaults to [`LATEST_YEAR`], `2024`).
    #[serde(default = "default_lodes_year")]
    pub year: u64,
    /// Fallback policy when a state/year combination is unavailable
    /// (defaults to `FallbackPolicy::TryPreviousYears(10)`).
    #[serde(default)]
    pub fallback: FallbackPolicy,
    /// WAC job categories/segments to retrieve (defaults to `[WacSegment::C000]`).
    #[serde(default = "default_wac_segments")]
    pub wac_segments: Vec<WacSegment>,
}

impl Default for LodesWacTigerConfig {
    fn default() -> Self {
        Self {
            study_region: None,
            data_granularity: default_lodes_granularity(),
            edition: LodesEdition::default(),
            job_type: LodesJobType::default(),
            segment: WorkplaceSegment::default(),
            year: default_lodes_year(),
            fallback: FallbackPolicy::default(),
            wac_segments: default_wac_segments(),
        }
    }
}

impl LodesWacTigerConfig {
    /// Assembles a [`LodesDataset::WAC`] from [`Self::edition`], [`Self::job_type`],
    /// [`Self::segment`], and [`Self::year`].
    pub fn to_dataset(&self) -> LodesDataset {
        LodesDataset::WAC {
            edition: self.edition,
            job_type: self.job_type,
            segment: self.segment,
            year: self.year,
        }
    }

    /// Returns the configured [`Self::study_region`], or [`RegionFilter::national`] if `None`.
    pub fn region_or_national(&self) -> RegionFilter {
        self.study_region
            .clone()
            .unwrap_or_else(RegionFilter::national)
    }

    /// Runs the LODES WAC + TIGER workflow using [`Self::study_region`] (or [`RegionFilter::national`] if `None`).
    pub async fn run(&self) -> Result<LodesTigerResponse, String> {
        let region = self.region_or_national();
        self.run_region(&region).await
    }

    /// Runs the LODES WAC + TIGER workflow for an explicit [`RegionFilter`].
    pub async fn run_region(&self, region: &RegionFilter) -> Result<LodesTigerResponse, String> {
        let dataset = self.to_dataset();
        lodes_tiger::run_region(
            region,
            &self.data_granularity,
            &self.wac_segments,
            &dataset,
            self.fallback,
        )
        .await
    }

    /// Runs the LODES WAC + TIGER workflow for an explicit [`RegionFilter`] using the supplied HTTP client.
    pub async fn run_region_with_client(
        &self,
        client: &ClientWithMiddleware,
        region: &RegionFilter,
    ) -> Result<LodesTigerResponse, String> {
        let dataset = self.to_dataset();
        lodes_tiger::run_region_with_client(
            client,
            region,
            &self.data_granularity,
            &self.wac_segments,
            &dataset,
            self.fallback,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bamcensus_core::model::identifier::Geoid;

    #[test]
    fn acs_tiger_config_default_matches_empty_json_deserialization() {
        let default_cfg = AcsTigerConfig::default();
        let from_empty_json: AcsTigerConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(default_cfg, from_empty_json);
        assert_eq!(default_cfg.acs_type, AcsType::FiveYear);
        assert_eq!(default_cfg.acs_year, 2025);
        assert_eq!(default_cfg.acs_resolution, GeoidType::BlockGroup);
        assert_eq!(default_cfg.acs_categories, vec!["B01001_001E".to_string()]);
        assert_eq!(default_cfg.api_token, None);

        let serialized = serde_json::to_string(&default_cfg).unwrap();
        let roundtrip: AcsTigerConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(default_cfg, roundtrip);
    }

    #[test]
    fn lodes_wac_tiger_config_default_matches_empty_json_deserialization() {
        let default_cfg = LodesWacTigerConfig::default();
        let from_empty_json: LodesWacTigerConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(default_cfg, from_empty_json);
        assert_eq!(default_cfg.study_region, None);
        assert_eq!(default_cfg.data_granularity, Some(GeoidType::BlockGroup));
        assert_eq!(default_cfg.edition, LodesEdition::Lodes8);
        assert_eq!(default_cfg.job_type, LodesJobType::JT00);
        assert_eq!(default_cfg.segment, WorkplaceSegment::S000);
        assert_eq!(default_cfg.year, LATEST_YEAR);
        assert_eq!(default_cfg.fallback, FallbackPolicy::TryPreviousYears(10));
        assert_eq!(default_cfg.wac_segments, vec![WacSegment::C000]);
        assert_eq!(
            default_cfg.to_dataset(),
            LodesDataset::WAC {
                edition: LodesEdition::Lodes8,
                job_type: LodesJobType::JT00,
                segment: WorkplaceSegment::S000,
                year: LATEST_YEAR,
            }
        );

        let serialized = serde_json::to_string(&default_cfg).unwrap();
        let roundtrip: LodesWacTigerConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(default_cfg, roundtrip);
    }

    #[test]
    fn configs_support_serde_flatten_in_downstream_structs() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct DownstreamWrapper {
            pub name: String,
            #[serde(flatten)]
            pub acs: AcsTigerConfig,
            #[serde(flatten)]
            pub lodes: LodesWacTigerConfig,
        }

        let json = r#"{
            "name": "denver_study",
            "acs_year": 2022,
            "acs_resolution": "census_tract",
            "year": 2021,
            "wac_segments": ["C000", "CNS07"]
        }"#;
        let parsed: DownstreamWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name, "denver_study");
        assert_eq!(parsed.acs.acs_year, 2022);
        assert_eq!(parsed.acs.acs_type, AcsType::FiveYear);
        assert_eq!(parsed.acs.acs_resolution, GeoidType::CensusTract);
        assert_eq!(parsed.lodes.year, 2021);
        assert_eq!(parsed.lodes.fallback, FallbackPolicy::TryPreviousYears(10));
        assert_eq!(
            parsed.lodes.wac_segments,
            vec![WacSegment::C000, WacSegment::CNS07]
        );
    }

    #[test]
    fn acs_config_converts_to_region_query_params() {
        let cfg = AcsTigerConfig {
            acs_type: AcsType::OneYear,
            acs_year: 2023,
            acs_resolution: GeoidType::County,
            acs_categories: vec!["NAME".to_string(), "B01001_001E".to_string()],
            api_token: Some("secret_token".to_string()),
        };
        let region = RegionFilter::Geoids(vec![Geoid::try_from("08").unwrap()]);
        let params = cfg.to_region_query_params(region.clone());
        assert_eq!(params.year, 2023);
        assert_eq!(params.acs_type, AcsType::OneYear);
        assert_eq!(params.output_resolution, Some(GeoidType::County));
        assert_eq!(params.region, region);
        assert_eq!(params.acs_token.as_deref(), Some("secret_token"));
    }
}
