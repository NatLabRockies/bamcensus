use crate::model::{TigerResource, TigerResourceBuilder};
use crate::ops::tiger_api::{get_geoid_from_record, read_resource};
use bamcensus_core::model::identifier::{fips, Geoid, GeoidType, HasGeoidString};
use bamcensus_core::model::spatial::{RegionFilter, SpatialPredicate};
use futures::{stream, StreamExt, TryStreamExt};
use geo::{Area, BooleanOps, BoundingRect, Centroid, Contains, Intersects, Relate};
use geo_types::{Geometry, MultiPolygon, Rect};
use reqwest::Client;
use shapefile::Shape;
use std::borrow::Cow;
use std::collections::HashSet;

/// Geometries selected at the requested level and conservative parent GEOIDs
/// suitable for broader data queries.
pub struct ResolvedRegion {
    pub matched_geometries: Vec<(Geoid, Geometry<f64>)>,
    pub parent_geoids: Vec<Geoid>,
}

/// Resolve a polygon to TIGER geometries and GEOIDs for the builder's vintage.
///
/// The calculation is planar in longitude/latitude coordinates, including
/// area fractions; it is not an equal-area or geodesic calculation. Inputs
/// must be topologically valid, and polygons crossing the antimeridian must
/// first be split there. Errors in any archive fail the whole resolution.
///
/// Coarse parent selection uses bounding boxes, not the requested predicate.
/// This is conservative even when a geometry's centroid lies outside it.
pub async fn resolve_polygon(
    client: &Client,
    builder: &TigerResourceBuilder,
    query_geom: &Geometry<f64>,
    target_type: GeoidType,
    predicate: SpatialPredicate,
) -> Result<ResolvedRegion, String> {
    let region = RegionFilter::new_geometry(query_geom.clone(), predicate)?;
    let RegionFilter::Geometry { geometry, .. } = region else {
        unreachable!()
    };
    let polygons = match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(multi) => multi.0,
        _ => unreachable!(),
    };
    for polygon in &polygons {
        if polygon.unsigned_area() <= 0.0 {
            return Err("query polygon must have positive area".to_string());
        }
        for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
            if ring.0.windows(2).any(|p| (p[0].x - p[1].x).abs() > 180.0) {
                return Err(
                    "query polygon crosses the antimeridian; split it at 180 degrees first"
                        .to_string(),
                );
            }
        }
    }
    // Union the components so overlapping input features represent a single region.
    let mut parts = polygons.into_iter();
    let mut query = MultiPolygon(vec![parts
        .next()
        .ok_or("query geometry must not be empty")?]);
    for polygon in parts {
        query = query.union(&MultiPolygon(vec![polygon]));
    }
    let bounds = query
        .bounding_rect()
        .ok_or("query geometry must not be empty")?;

    let states = if matches!(builder, TigerResourceBuilder::Tiger2010)
        || target_type == GeoidType::Place
        || target_type == GeoidType::State
    {
        let all_states = if matches!(builder, TigerResourceBuilder::Tiger2010) {
            let mut states = Geoid::all_states();
            states.extend([60, 66, 69, 72, 78].map(|s| Geoid::State(fips::State(s))));
            states
        } else {
            vec![Geoid::State(fips::State(1))]
        };
        let resources = resources_for(builder, GeoidType::State, &all_states)?;
        let mode = if target_type == GeoidType::State {
            Some(predicate)
        } else {
            None
        };
        let rows = load_resources(client, resources, &query, bounds, mode).await?;
        if target_type == GeoidType::State {
            return Ok(result_from_matches(rows));
        }
        rows.into_iter().map(|(geoid, _)| geoid).collect()
    } else {
        Vec::new()
    };

    if target_type == GeoidType::Place {
        let resources = resources_for(builder, target_type, &states)?;
        let rows = load_resources(client, resources, &query, bounds, Some(predicate)).await?;
        return Ok(ResolvedRegion {
            matched_geometries: rows,
            parent_geoids: states,
        });
    }

    let county_scopes = if matches!(builder, TigerResourceBuilder::Tiger2010) {
        states
    } else {
        vec![Geoid::State(fips::State(1))]
    };
    let resources = resources_for(builder, GeoidType::County, &county_scopes)?;
    let mode = if target_type == GeoidType::County {
        Some(predicate)
    } else {
        None
    };
    let counties = load_resources(client, resources, &query, bounds, mode).await?;
    if target_type == GeoidType::County {
        return Ok(result_from_matches(counties));
    }
    let parent_geoids: Vec<_> = counties.into_iter().map(|(geoid, _)| geoid).collect();
    let resources = resources_for(builder, target_type, &parent_geoids)?;
    let matched_geometries =
        load_resources(client, resources, &query, bounds, Some(predicate)).await?;
    Ok(ResolvedRegion {
        matched_geometries,
        parent_geoids,
    })
}

/// Resolve a polygon to GEOIDs without returning their geometries.
pub async fn resolve_geoids(
    client: &Client,
    builder: &TigerResourceBuilder,
    query_geom: &Geometry<f64>,
    target_type: GeoidType,
    predicate: SpatialPredicate,
) -> Result<Vec<Geoid>, String> {
    Ok(
        resolve_polygon(client, builder, query_geom, target_type, predicate)
            .await?
            .matched_geometries
            .into_iter()
            .map(|(geoid, _)| geoid)
            .collect(),
    )
}

fn result_from_matches(rows: Vec<(Geoid, Geometry<f64>)>) -> ResolvedRegion {
    let parent_geoids = rows.iter().map(|(geoid, _)| geoid.clone()).collect();
    ResolvedRegion {
        matched_geometries: rows,
        parent_geoids,
    }
}

fn resources_for(
    builder: &TigerResourceBuilder,
    target: GeoidType,
    parents: &[Geoid],
) -> Result<Vec<TigerResource>, String> {
    let mut resources = HashSet::new();
    for parent in parents {
        let Geoid::State(state) = parent.to_state() else {
            unreachable!()
        };
        let county = match parent {
            Geoid::County(_, county) => *county,
            _ => fips::County(0),
        };
        let child = match target {
            GeoidType::State => Geoid::State(state),
            GeoidType::County => Geoid::County(state, county),
            GeoidType::CountySubdivision => {
                Geoid::CountySubdivision(state, county, fips::CountySubdivision(0))
            }
            GeoidType::Place => Geoid::Place(state, fips::Place(0)),
            GeoidType::CensusTract => Geoid::CensusTract(state, county, fips::CensusTract(0)),
            GeoidType::BlockGroup => {
                Geoid::BlockGroup(state, county, fips::CensusTract(0), fips::BlockGroup(0))
            }
            GeoidType::Block => Geoid::Block(
                state,
                county,
                fips::CensusTract(0),
                fips::Block("0000".to_string()),
            ),
        };
        resources.insert(builder.create_resource(&child)?);
    }
    let mut resources: Vec<_> = resources.into_iter().collect();
    resources.sort_by(|a, b| a.uri.cmp(&b.uri));
    Ok(resources)
}

async fn load_resources(
    client: &Client,
    resources: Vec<TigerResource>,
    query: &MultiPolygon<f64>,
    bounds: Rect<f64>,
    predicate: Option<SpatialPredicate>,
) -> Result<Vec<(Geoid, Geometry<f64>)>, String> {
    let rows = stream::iter(resources.into_iter().map(|resource| async move {
        read_resource(client, &resource, |shape, record| {
            let shape_bounds = shape_bounds(&shape)?;
            if !shape_bounds.intersects(&bounds) {
                return Ok(None);
            }
            let geoid = get_geoid_from_record(&record, &resource.geoid_type)?;
            let geometry: Geometry<f64> = shape
                .try_into()
                .map_err(|e| format!("cannot convert TIGER geometry for {geoid}: {e}"))?;
            if let Some(predicate) = predicate {
                if !matches_predicate(query, &geometry, predicate)? {
                    return Ok(None);
                }
            }
            Ok(Some((geoid, geometry)))
        })
        .await
        .map_err(|e| format!("{}: {e}", resource.uri))
    }))
    .buffer_unordered(8)
    .try_collect::<Vec<_>>()
    .await?;
    let mut rows: Vec<_> = rows.into_iter().flatten().collect();
    rows.sort_by_key(|(geoid, _)| geoid.geoid_string());
    Ok(rows)
}

fn shape_bounds(shape: &Shape) -> Result<Rect<f64>, String> {
    let (x, y) = match shape {
        Shape::Polygon(p) => (p.bbox().x_range(), p.bbox().y_range()),
        Shape::PolygonM(p) => (p.bbox().x_range(), p.bbox().y_range()),
        Shape::PolygonZ(p) => (p.bbox().x_range(), p.bbox().y_range()),
        _ => return Err("TIGER record does not contain a polygon".to_string()),
    };
    Ok(Rect::new((x[0], y[0]), (x[1], y[1])))
}

fn matches_predicate(
    query: &MultiPolygon<f64>,
    geometry: &Geometry<f64>,
    predicate: SpatialPredicate,
) -> Result<bool, String> {
    let candidate = match geometry {
        Geometry::Polygon(p) => Cow::Owned(MultiPolygon(vec![p.clone()])),
        Geometry::MultiPolygon(p) => Cow::Borrowed(p),
        _ => return Err("TIGER geometry is not polygonal".to_string()),
    };
    Ok(match predicate {
        SpatialPredicate::Intersects => query.intersects(candidate.as_ref()),
        SpatialPredicate::CentroidWithin => {
            let centroid = candidate
                .centroid()
                .ok_or("TIGER geometry has no centroid")?;
            query.contains(&centroid)
        }
        SpatialPredicate::Within => candidate.relate(query).is_coveredby(),
        SpatialPredicate::AreaOverlap { min_fraction } => {
            let area = candidate.unsigned_area();
            if area <= 0.0 || !area.is_finite() {
                return Err("TIGER geometry has zero or invalid area".to_string());
            }
            candidate.intersection(query).unsigned_area() / area >= min_fraction
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{polygon, Polygon};

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon<f64> {
        polygon![(x: x0, y: y0), (x: x1, y: y0), (x: x1, y: y1), (x: x0, y: y1), (x: x0, y: y0)]
    }

    #[test]
    fn predicates_handle_touching_containment_and_overlap_thresholds() {
        let query = MultiPolygon(vec![square(0.0, 0.0, 2.0, 2.0)]);
        let inside = Geometry::Polygon(square(0.0, 0.0, 1.0, 1.0));
        let half = Geometry::Polygon(square(1.0, 0.0, 3.0, 2.0));
        let touching = Geometry::Polygon(square(2.0, 0.0, 3.0, 1.0));
        assert!(matches_predicate(&query, &inside, SpatialPredicate::Within).unwrap());
        assert!(!matches_predicate(&query, &half, SpatialPredicate::Within).unwrap());
        assert!(matches_predicate(&query, &touching, SpatialPredicate::Intersects).unwrap());
        assert!(!matches_predicate(&query, &touching, SpatialPredicate::CentroidWithin).unwrap());
        assert!(matches_predicate(
            &query,
            &half,
            SpatialPredicate::AreaOverlap { min_fraction: 0.5 }
        )
        .unwrap());
        assert!(!matches_predicate(
            &query,
            &half,
            SpatialPredicate::AreaOverlap {
                min_fraction: 0.5001
            }
        )
        .unwrap());
        assert!(!matches_predicate(
            &query,
            &touching,
            SpatialPredicate::AreaOverlap { min_fraction: 0.01 }
        )
        .unwrap());
        // The centroid is on the query boundary, not in its interior.
        assert!(!matches_predicate(&query, &half, SpatialPredicate::CentroidWithin).unwrap());
    }

    #[test]
    fn resource_scopes_follow_vintage_and_deduplicate() {
        let counties = vec![
            Geoid::try_from("08031").unwrap(),
            Geoid::try_from("08059").unwrap(),
        ];
        let old = TigerResourceBuilder::new(2010).unwrap();
        let new = TigerResourceBuilder::new(2020).unwrap();
        let old_subdivisions =
            resources_for(&old, GeoidType::CountySubdivision, &counties).unwrap();
        assert_eq!(old_subdivisions.len(), 2);
        assert!(old_subdivisions
            .iter()
            .all(|r| r.file_scope == Some(GeoidType::County)));
        assert_eq!(
            resources_for(&new, GeoidType::CensusTract, &counties)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            resources_for(&old, GeoidType::CensusTract, &counties)
                .unwrap()
                .len(),
            2
        );
        assert!(resources_for(&new, GeoidType::Block, &[])
            .unwrap()
            .is_empty());
    }
}
