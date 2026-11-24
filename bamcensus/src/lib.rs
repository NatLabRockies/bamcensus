//! BAMCENSUS
//!
//! Rust API for the Behavior and Advanced Mobility Census dataset aggregator.
pub mod app;
pub mod model;
pub mod ops;

#[cfg(all(test, feature = "schemars"))]
mod schemars_tests {
    use crate::app::acs_tiger::AcsRegionQueryParams;
    use bamcensus_acs::model::{AcsApiQueryParams, AcsGeoidQuery, AcsType};
    use bamcensus_core::model::identifier::{Geoid, GeoidType, StateCode};
    use bamcensus_core::model::spatial::{RegionFilter, SpatialPredicate};
    use bamcensus_lehd::model::{
        FallbackPolicy, LodesDataset, LodesEdition, LodesJobType, WacSegment, WorkplaceSegment,
    };
    use schemars::schema_for;

    #[test]
    fn generates_json_schemas_for_all_supported_models() {
        let acs_type_schema = serde_json::to_value(schema_for!(AcsType)).unwrap();
        assert_eq!(
            acs_type_schema["enum"],
            serde_json::json!(["one_year", "five_year"])
        );

        let geoid_type_schema = serde_json::to_value(schema_for!(GeoidType)).unwrap();
        assert_eq!(
            geoid_type_schema["enum"],
            serde_json::json!([
                "state",
                "county",
                "county_subdivision",
                "place",
                "census_tract",
                "block_group",
                "block"
            ])
        );

        let fallback_schema = serde_json::to_value(schema_for!(FallbackPolicy)).unwrap();
        let fallback_json = fallback_schema.to_string();
        assert!(fallback_json.contains("FailAlways"));
        assert!(fallback_json.contains("TryPreviousYears"));
        assert!(fallback_json.contains("TryNextYears"));
        assert!(fallback_json.contains("BiDirectional"));

        let geoid_schema = serde_json::to_value(schema_for!(Geoid)).unwrap();
        assert_eq!(geoid_schema["type"], serde_json::json!("string"));

        // Verify all other requested types generate valid non-empty schemas
        for schema in [
            geoid_schema,
            serde_json::to_value(schema_for!(StateCode)).unwrap(),
            serde_json::to_value(schema_for!(SpatialPredicate)).unwrap(),
            serde_json::to_value(schema_for!(RegionFilter)).unwrap(),
            serde_json::to_value(schema_for!(AcsRegionQueryParams)).unwrap(),
            serde_json::to_value(schema_for!(AcsApiQueryParams)).unwrap(),
            serde_json::to_value(schema_for!(AcsGeoidQuery)).unwrap(),
            serde_json::to_value(schema_for!(LodesDataset)).unwrap(),
            serde_json::to_value(schema_for!(LodesEdition)).unwrap(),
            serde_json::to_value(schema_for!(LodesJobType)).unwrap(),
            serde_json::to_value(schema_for!(WacSegment)).unwrap(),
            serde_json::to_value(schema_for!(WorkplaceSegment)).unwrap(),
        ] {
            assert!(schema.is_object());
        }
    }
}
