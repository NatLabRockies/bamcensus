use bamcensus_core::model::spatial::{RegionFilter, SpatialPredicate};
use geo_types::{Geometry, GeometryCollection};
use geojson::GeoJson;
use std::{fs, path::Path};
use wkt::TryFromWkt;

/// Read a polygonal region from inline WKT or GeoJSON, or a WKT, GeoJSON, or
/// shapefile path. Coordinates must already be longitude/latitude in EPSG:4269
/// or EPSG:4326; files are not reprojected.
pub fn parse_geometry_input(input: &str) -> Result<Geometry<f64>, String> {
    let input = input.trim();
    let geometry = if input.starts_with('{') {
        parse_geojson(input)?
    } else if !input.to_ascii_lowercase().ends_with(".wkt")
        && !input.to_ascii_lowercase().ends_with(".geojson")
        && !input.to_ascii_lowercase().ends_with(".json")
        && !input.to_ascii_lowercase().ends_with(".shp")
        && (input
            .split_once('(')
            .is_some_and(|(prefix, _)| !prefix.contains(['/', '\\']))
            || input.to_ascii_uppercase().ends_with(" EMPTY"))
    {
        Geometry::try_from_wkt_str(input).map_err(|e| format!("invalid WKT geometry: {e}"))?
    } else {
        let path = Path::new(input);
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match extension.to_ascii_lowercase().as_str() {
            "shp" => {
                let mut reader = shapefile::ShapeReader::from_path(path)
                    .map_err(|e| format!("cannot open shapefile {}: {e}", path.display()))?;
                let geometries = reader
                    .iter_shapes()
                    .map(|shape| {
                        let shape = shape.map_err(|e| format!("cannot read shapefile: {e}"))?;
                        Geometry::try_from(shape)
                            .map_err(|e| format!("cannot convert shapefile geometry: {e}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Geometry::GeometryCollection(GeometryCollection(geometries))
            }
            "wkt" => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("cannot read WKT file {}: {e}", path.display()))?;
                Geometry::try_from_wkt_str(&content)
                    .map_err(|e| format!("invalid WKT geometry in {}: {e}", path.display()))?
            }
            "geojson" | "json" => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("cannot read GeoJSON file {}: {e}", path.display()))?;
                parse_geojson(&content)?
            }
            _ => {
                return Err(format!(
                    "expected WKT, GeoJSON, or a .wkt, .geojson, .json, or .shp path: {input}"
                ))
            }
        }
    };
    match RegionFilter::new_geometry(geometry, SpatialPredicate::Intersects)? {
        RegionFilter::Geometry { geometry, .. } => Ok(geometry),
        RegionFilter::Geoids(_) => unreachable!(),
    }
}

fn parse_geojson(input: &str) -> Result<Geometry<f64>, String> {
    let geojson = input
        .parse::<GeoJson>()
        .map_err(|e| format!("invalid GeoJSON: {e}"))?;
    let convert = |geometry: geojson::Geometry| {
        Geometry::try_from(geometry).map_err(|e| format!("invalid GeoJSON geometry: {e}"))
    };
    let feature_geometry = |feature: geojson::Feature| {
        feature
            .geometry
            .ok_or_else(|| "GeoJSON feature has no geometry".to_string())
            .and_then(convert)
    };
    match geojson {
        GeoJson::Geometry(geometry) => convert(geometry),
        GeoJson::Feature(feature) => feature_geometry(feature),
        GeoJson::FeatureCollection(collection) => {
            let geometries = collection
                .features
                .into_iter()
                .map(feature_geometry)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Geometry::GeometryCollection(GeometryCollection(geometries)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wkt_geojson_and_file() {
        let wkt = "POLYGON ((0 0, 1 0, 1 1, 0 0))";
        assert!(matches!(
            parse_geometry_input(wkt).unwrap(),
            Geometry::Polygon(_)
        ));
        let json = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}}]}"#;
        assert!(matches!(
            parse_geometry_input(json).unwrap(),
            Geometry::MultiPolygon(_)
        ));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("region.geojson");
        fs::write(&path, json).unwrap();
        assert!(parse_geometry_input(path.to_str().unwrap()).is_ok());
    }

    #[test]
    fn rejects_missing_or_non_polygonal_input() {
        assert!(parse_geometry_input("POINT (0 0)").is_err());
        assert!(
            parse_geometry_input(r#"{"type":"Feature","geometry":null,"properties":{}}"#).is_err()
        );
        assert!(parse_geometry_input("missing.wkt").is_err());
        assert!(parse_geometry_input("POLYGON EMPTY").is_err());
    }
}
