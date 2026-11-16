use bamcensus_core::model::identifier::{Geoid, GeoidType, StateCode};
use bamcensus_core::model::spatial::{RegionFilter, SpatialPredicate};
use bamcensus_tiger::ops::geometry_io::parse_geometry_input;
use clap::Args;
use itertools::Itertools;
use std::str::FromStr;

#[derive(Args, Debug, Clone, Default)]
pub struct RegionCliArgs {
    /// geoid or comma-delimited list of geoids
    #[arg(short, long, alias = "geoid", conflicts_with = "polygon")]
    pub geoids: Option<String>,
    /// polygonal query region (inline WKT/GeoJSON, or path to .wkt, .geojson, .json, or .shp file)
    #[arg(long, conflicts_with = "geoids")]
    pub polygon: Option<String>,
    /// spatial predicate for polygon queries: intersects, centroid-within, within, or area-overlap[:fraction]
    #[arg(long, default_value = "intersects")]
    pub spatial_predicate: String,
    /// minimum overlap fraction in (0, 1] when --spatial-predicate is area-overlap
    #[arg(long)]
    pub min_overlap: Option<f64>,
}

impl RegionCliArgs {
    pub fn parse_predicate(&self) -> Result<SpatialPredicate, String> {
        let raw = self.spatial_predicate.trim();
        let is_bare_area_overlap = matches!(raw, "area_overlap" | "area-overlap");
        let is_prefixed_area_overlap =
            raw.starts_with("area_overlap:") || raw.starts_with("area-overlap:");

        if let Some(min_fraction) = self.min_overlap {
            if is_bare_area_overlap {
                let predicate = SpatialPredicate::AreaOverlap { min_fraction };
                predicate.validate()?;
                return Ok(predicate);
            } else if is_prefixed_area_overlap {
                return Err(
                    "specify overlap threshold via either --spatial-predicate area-overlap:<fraction> or --min-overlap <fraction>, not both"
                        .to_string(),
                );
            } else {
                return Err(format!(
                    "--min-overlap can only be used with --spatial-predicate area-overlap (found '{raw}')"
                ));
            }
        }

        if is_bare_area_overlap {
            return Err(
                "area-overlap predicate requires a threshold via --spatial-predicate area-overlap:<fraction> or --min-overlap <fraction>"
                    .to_string(),
            );
        }

        SpatialPredicate::from_str(raw)
    }

    pub fn to_region_filter(
        &self,
        output_resolution: Option<GeoidType>,
        allow_default_all_states: bool,
    ) -> Result<RegionFilter, String> {
        if self.geoids.is_some() && self.polygon.is_some() {
            return Err("cannot specify both --geoids and --polygon".to_string());
        }

        if let Some(polygon_input) = &self.polygon {
            if output_resolution.is_none() {
                return Err(
                    "--output-resolution is required when querying with --polygon".to_string(),
                );
            }
            let predicate = self.parse_predicate()?;
            let geometry = parse_geometry_input(polygon_input)?;
            return RegionFilter::new_geometry(geometry, predicate);
        }

        if let Some(geoids_str) = &self.geoids {
            let trimmed = geoids_str.trim();
            if trimmed.is_empty() {
                return Err("--geoids must not be empty".to_string());
            }
            let geoids = trimmed
                .split(',')
                .map(|s| Geoid::try_from(s.trim()))
                .collect::<Result<Vec<Geoid>, _>>()?;
            return Ok(RegionFilter::Geoids(geoids));
        }

        if allow_default_all_states {
            let geoids = StateCode::ALL
                .iter()
                .map(|sc| {
                    let fips = sc.to_fips_string();
                    Geoid::try_from(fips.as_str()).unwrap()
                })
                .collect_vec();
            Ok(RegionFilter::Geoids(geoids))
        } else {
            Err("either --geoids or --polygon must be provided".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_predicates_and_min_overlap_combinations() {
        let mut args = RegionCliArgs {
            geoids: None,
            polygon: Some("POLYGON ((-105 39, -104 39, -104 40, -105 40, -105 39))".to_string()),
            spatial_predicate: "area-overlap".to_string(),
            min_overlap: Some(0.25),
        };
        assert_eq!(
            args.parse_predicate().unwrap(),
            SpatialPredicate::AreaOverlap { min_fraction: 0.25 }
        );

        args.min_overlap = None;
        assert!(args.parse_predicate().is_err());

        args.spatial_predicate = "area-overlap:0.5".to_string();
        assert_eq!(
            args.parse_predicate().unwrap(),
            SpatialPredicate::AreaOverlap { min_fraction: 0.5 }
        );

        args.min_overlap = Some(0.5);
        assert!(args.parse_predicate().is_err());

        args.spatial_predicate = "within".to_string();
        assert!(args.parse_predicate().is_err());
    }

    #[test]
    fn builds_region_filter_for_geoids_polygon_and_defaults() {
        let geoid_args = RegionCliArgs {
            geoids: Some("08031,08059".to_string()),
            polygon: None,
            spatial_predicate: "intersects".to_string(),
            min_overlap: None,
        };
        let filter = geoid_args.to_region_filter(None, false).unwrap();
        assert!(matches!(filter, RegionFilter::Geoids(g) if g.len() == 2));

        let polygon_args = RegionCliArgs {
            geoids: None,
            polygon: Some("POLYGON ((-105 39, -104 39, -104 40, -105 40, -105 39))".to_string()),
            spatial_predicate: "centroid-within".to_string(),
            min_overlap: None,
        };
        assert!(polygon_args.to_region_filter(None, false).is_err());
        let filter = polygon_args
            .to_region_filter(Some(GeoidType::CensusTract), false)
            .unwrap();
        assert!(matches!(
            filter,
            RegionFilter::Geometry {
                predicate: SpatialPredicate::CentroidWithin,
                ..
            }
        ));

        let empty_args = RegionCliArgs::default();
        assert!(empty_args.to_region_filter(None, false).is_err());
        let all_states = empty_args.to_region_filter(None, true).unwrap();
        assert!(matches!(all_states, RegionFilter::Geoids(g) if g.len() == 51));
    }
}
