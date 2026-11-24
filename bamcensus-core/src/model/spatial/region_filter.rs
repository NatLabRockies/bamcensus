use super::SpatialPredicate;
use crate::model::identifier::{Geoid, StateCode};
use geo_types::{Geometry, MultiPolygon, Polygon, Rect};
use serde::{Deserialize, Serialize};

/// A selection of explicit GEOIDs or a geographic query region.
///
/// Geometry coordinates must be longitude/latitude in EPSG:4269 or EPSG:4326.
/// Call [`Self::validate`] on directly constructed or deserialized values before use.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum RegionFilter {
    /// Explicit geographic identifiers.
    Geoids(Vec<Geoid>),
    /// Select geometries using a polygonal region and a spatial predicate.
    Geometry {
        #[cfg_attr(feature = "schemars", schemars(with = "serde_json::Value"))]
        geometry: Geometry<f64>,
        predicate: SpatialPredicate,
    },
}

impl RegionFilter {
    /// Create a national region filter covering all 50 US states and the District of Columbia.
    pub fn national() -> Self {
        Self::Geoids(StateCode::all_state_geoids())
    }

    /// Create a polygonal region filter from a longitude/latitude bounding box (`Rect<f64>`).
    pub fn from_rect(rect: Rect<f64>, predicate: SpatialPredicate) -> Result<Self, String> {
        let min = rect.min();
        let max = rect.max();
        if !min.x.is_finite()
            || !min.y.is_finite()
            || !max.x.is_finite()
            || !max.y.is_finite()
            || min.x >= max.x
            || min.y >= max.y
        {
            return Err("query bounding box must have finite min < max coordinates".to_string());
        }
        Self::new_geometry(Geometry::Polygon(rect.to_polygon()), predicate)
    }

    /// Validate a polygonal region and normalize geometry collections to a multipolygon.
    ///
    /// Coordinate ranges do not establish the CRS, and this does not validate polygon topology.
    /// Collection members are collected, not geometrically unioned.
    pub fn new_geometry(
        geometry: Geometry<f64>,
        predicate: SpatialPredicate,
    ) -> Result<Self, String> {
        predicate.validate()?;
        let geometry = match geometry {
            Geometry::GeometryCollection(collection) => {
                let mut polygons = Vec::new();
                for geometry in collection.0 {
                    collect_polygons(geometry, &mut polygons)?;
                }
                Geometry::MultiPolygon(MultiPolygon(polygons))
            }
            other => other,
        };
        let region = Self::Geometry {
            geometry,
            predicate,
        };
        region.validate()?;
        Ok(region)
    }

    /// Validate the predicate and polygonal longitude/latitude coordinates.
    ///
    /// This checks neither CRS metadata nor polygon topology.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Geoids(_) => Ok(()),
            Self::Geometry {
                geometry,
                predicate,
            } => {
                predicate.validate()?;
                validate_geometry(geometry)
            }
        }
    }
}

fn collect_polygons(
    geometry: Geometry<f64>,
    polygons: &mut Vec<Polygon<f64>>,
) -> Result<(), String> {
    match geometry {
        Geometry::Polygon(polygon) => polygons.push(polygon),
        Geometry::MultiPolygon(multi) => polygons.extend(multi.0),
        Geometry::GeometryCollection(collection) => {
            for geometry in collection.0 {
                collect_polygons(geometry, polygons)?;
            }
        }
        _ => return Err("query geometry must contain only polygons or multipolygons".to_string()),
    }
    Ok(())
}

fn validate_geometry(geometry: &Geometry<f64>) -> Result<(), String> {
    match geometry {
        Geometry::Polygon(polygon) => validate_polygon(polygon),
        Geometry::MultiPolygon(multi) if !multi.0.is_empty() => {
            for polygon in &multi.0 {
                validate_polygon(polygon)?;
            }
            Ok(())
        }
        Geometry::MultiPolygon(_) => Err("query geometry must not be empty".to_string()),
        _ => Err("query geometry must be a polygon or multipolygon".to_string()),
    }
}

fn validate_polygon(polygon: &Polygon<f64>) -> Result<(), String> {
    if polygon.exterior().0.is_empty() {
        return Err("query polygon must not be empty".to_string());
    }
    for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
        for coordinate in &ring.0 {
            if !coordinate.x.is_finite()
                || !coordinate.y.is_finite()
                || !(-180.0..=180.0).contains(&coordinate.x)
                || !(-90.0..=90.0).contains(&coordinate.y)
            {
                return Err(
                    "query geometry must use finite longitude/latitude coordinates in EPSG:4269 or EPSG:4326"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{coord, GeometryCollection, LineString, Point};

    fn polygon(x: f64, y: f64) -> Polygon<f64> {
        Polygon::new(
            LineString::from(vec![(x, y), (x + 1.0, y), (x, y + 1.0), (x, y)]),
            vec![],
        )
    }

    #[test]
    fn constructs_and_serializes_geometry_and_geoids() {
        let region = RegionFilter::new_geometry(
            Geometry::Polygon(polygon(-105.0, 39.0)),
            SpatialPredicate::Intersects,
        )
        .unwrap();
        for value in [
            region,
            RegionFilter::Geoids(vec![Geoid::try_from("08").unwrap()]),
        ] {
            let json = serde_json::to_string(&value).unwrap();
            let decoded: RegionFilter = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, value);
            decoded.validate().unwrap();
        }
    }

    #[test]
    fn normalizes_polygonal_collections() {
        let collection = Geometry::GeometryCollection(GeometryCollection(vec![
            Geometry::Polygon(polygon(-105.0, 39.0)),
            Geometry::GeometryCollection(GeometryCollection(vec![Geometry::MultiPolygon(
                MultiPolygon(vec![polygon(-104.0, 39.0)]),
            )])),
        ]));
        let region = RegionFilter::new_geometry(collection, SpatialPredicate::Within).unwrap();
        assert!(matches!(
            region,
            RegionFilter::Geometry { geometry: Geometry::MultiPolygon(MultiPolygon(polygons)), .. }
                if polygons.len() == 2
        ));
    }

    #[test]
    fn rejects_empty_nonpolygonal_and_invalid_coordinates() {
        let polygon_with_bad_hole = Polygon::new(
            polygon(-105.0, 39.0).exterior().clone(),
            vec![LineString::from(vec![coord! { x: 181.0, y: 0.0 }])],
        );
        for geometry in [
            Geometry::Polygon(Polygon::new(LineString::new(vec![]), vec![])),
            Geometry::MultiPolygon(MultiPolygon(vec![])),
            Geometry::GeometryCollection(GeometryCollection(vec![])),
            Geometry::Point(Point::new(0.0, 0.0)),
            Geometry::GeometryCollection(GeometryCollection(vec![Geometry::Point(Point::new(
                0.0, 0.0,
            ))])),
            Geometry::Polygon(polygon(500_000.0, 4_000_000.0)),
            Geometry::Polygon(polygon(0.0, f64::NAN)),
            Geometry::Polygon(polygon_with_bad_hole),
        ] {
            assert!(RegionFilter::new_geometry(geometry, SpatialPredicate::Intersects).is_err());
        }
    }

    #[test]
    fn validates_directly_constructed_values() {
        let geometry = Geometry::Polygon(polygon(-105.0, 39.0));
        let predicate = SpatialPredicate::AreaOverlap { min_fraction: 0.0 };
        assert!(RegionFilter::new_geometry(geometry.clone(), predicate).is_err());
        assert!(RegionFilter::Geometry {
            geometry,
            predicate
        }
        .validate()
        .is_err());
    }

    #[test]
    fn constructs_national_and_rect_regions() {
        let national = RegionFilter::national();
        assert!(matches!(&national, RegionFilter::Geoids(geoids) if geoids.len() == 51));
        national.validate().unwrap();

        let rect = Rect::new(coord! { x: -105.5, y: 39.5 }, coord! { x: -104.5, y: 40.5 });
        let region = RegionFilter::from_rect(rect, SpatialPredicate::Intersects).unwrap();
        region.validate().unwrap();
        assert!(matches!(
            region,
            RegionFilter::Geometry {
                geometry: Geometry::Polygon(_),
                predicate: SpatialPredicate::Intersects
            }
        ));

        let degenerate = Rect::new(coord! { x: -105.0, y: 39.5 }, coord! { x: -105.0, y: 40.5 });
        assert!(RegionFilter::from_rect(degenerate, SpatialPredicate::Intersects).is_err());
    }
}
