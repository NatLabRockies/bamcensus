use crate::model::lodes_wac_tiger_row::LodesWacTigerRow;
use bamcensus_core::model::identifier::Geoid;
use bamcensus_core::model::identifier::GeoidType;
use bamcensus_core::model::spatial::RegionFilter;
use bamcensus_lehd::api::lodes_api;
use bamcensus_lehd::model::{FallbackPolicy, LodesDataset, LodesQuery, WacSegment, WacValue};
use bamcensus_tiger::model::TigerResourceBuilder;
use bamcensus_tiger::ops::{spatial_resolver, tiger_api};
use geo::Geometry;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LodesTigerResponse {
    pub join_dataset: Vec<LodesWacTigerRow>,
    pub tiger_errors: Vec<String>,
    pub join_errors: Vec<String>,
}

/// runs a LODES WAC query combined with TIGER geography data. the result will include a list of
/// GEOIDs alongside LODES workplace characteristics (WAC) data. all GEOIDs are used to run a set
/// of downloads from both the LODES dataset and TIGER/Lines datasets. the geometries from TIGER
/// are combined with the LODES WAC data producing LodesWacTigerRows.
///
/// # Arguments
///
/// * `geoids` - List of GEOIDs to query; if empty, all states are used
/// * `agg_geoid_type` - Optional aggregation level for output
/// * `wac_segments` - WAC segments to retrieve from LODES
/// * `dataset` - LODES dataset specification (year, edition, job type, segment)
/// * `fallback_policy` - Fallback policy for handling unavailable state/year combinations
///
/// # Fallback Policy
///
/// When LODES data for a requested state/year is unavailable (404 Not Found), the fallback_policy
/// determines fallback behavior:
///
/// - `FallbackPolicy::FailAlways` - Fail immediately if year is unavailable (default)
/// - `FallbackPolicy::TryPreviousYears(n)` - Try N earlier years before failing
/// - `FallbackPolicy::TryNextYears(n)` - Try N later years before failing
/// - `FallbackPolicy::BiDirectional { past: n, future: m }` - Try N earlier or M later years
///
/// # Example
///
/// ```no_run
/// use bamcensus::app::lodes_tiger::run;
/// use bamcensus_lehd::model::{LodesDataset, LodesEdition, LodesJobType, FallbackPolicy, WacSegment, WorkplaceSegment};
/// use bamcensus_core::model::identifier::Geoid;
/// use bamcensus_core::model::identifier::GeoidType;
///
/// let geoids = vec![Geoid::try_from("08").unwrap()];
/// let wildcard = Some(GeoidType::County);
/// let wac_segments = vec![WacSegment::C000];
/// let dataset = LodesDataset::WAC {
///     edition: LodesEdition::Lodes8,
///     job_type: LodesJobType::JT00,
///     segment: WorkplaceSegment::S000,
///     year: 2020,
/// };
/// let fallback_policy = FallbackPolicy::TryPreviousYears(2);
///
/// # tokio_test::block_on(async {
///     let res = run(&geoids, &wildcard, &wac_segments, &dataset, fallback_policy).await.unwrap();
///     println!(
///         "found {} responses, {} errors",
///         res.join_dataset.len(),
///         res.tiger_errors.len() + res.join_errors.len(),
///     );
///     for row in res.join_dataset.into_iter() {
///         println!("{}", row)
///     }
/// # })
///
/// ```
pub async fn run(
    geoids: &Vec<Geoid>,
    agg_geoid_type: &Option<GeoidType>,
    wac_segments: &[WacSegment],
    dataset: &LodesDataset,
    fallback_policy: FallbackPolicy,
) -> Result<LodesTigerResponse, String> {
    // input: i have a set of geoids that describe a region. i want to download
    // lodes data and aggregate it to some GeoidType.
    // use the LODES dataset argument to build URIs for all LODES downloads
    // if the user did not provide geoids, use all states
    let filter_geoids = match geoids.len() {
        0 => Geoid::all_states(),
        _ => geoids.clone(),
    };
    let state_geoids = filter_geoids
        .iter()
        .map(|g| g.to_state())
        .unique()
        .collect_vec();
    let lodes_queries = state_geoids
        .iter()
        .map(|geoid| dataset.create_query(geoid))
        .collect::<Result<Vec<_>, _>>()?;

    let agg_fn = bamcensus_core::ops::agg::NumericAggregation::Sum;
    let agg = agg_geoid_type.map(|g| (g, agg_fn));

    // execute LODES downloads with fallback policy

    let client: Client = Client::new();
    let lodes_rows = lodes_api::run_wac_with_fallback(
        &client,
        &lodes_queries,
        wac_segments,
        agg,
        fallback_policy,
    )
    .await?;

    // filter result. LODES collects by State. here we only accept rows that match
    // the input geoids or where the input geoids are the (FIPS hierarchical) parent.
    let lodes_filtered = filter_rows_by_geoids(lodes_rows, &filter_geoids);

    // execute TIGER/Lines downloads selecting a data vintage based on the LODES edition chosen
    let tiger_year = dataset.tiger_year();
    let tiger_uri_builder = TigerResourceBuilder::new(tiger_year)?;
    let lodes_geoids = &lodes_filtered.iter().map(|(geoid, _)| geoid).collect_vec();
    let tiger_response = tiger_api::run(&client, &tiger_uri_builder, lodes_geoids).await?;

    type NestedResult = (Vec<Vec<(Geoid, Geometry<f64>)>>, Vec<String>);
    let (tiger_rows_nested, tiger_errors): NestedResult =
        tiger_response.into_iter().partition_result();

    finish_response(lodes_filtered, tiger_rows_nested, tiger_errors)
}

/// Run a LODES WAC query for explicit GEOIDs or a polygonal region.
///
/// Polygon selection applies at the output resolution (block by default).
/// Aggregated values include all blocks in each selected geography; they are
/// not clipped or weighted by the polygon. Place and county subdivision
/// aggregation are not supported. Territories bamcensus cannot query are
/// omitted and reported in `tiger_errors`. Resolver archive errors fail the query.
pub async fn run_region(
    region: &RegionFilter,
    agg_geoid_type: &Option<GeoidType>,
    wac_segments: &[WacSegment],
    dataset: &LodesDataset,
    fallback_policy: FallbackPolicy,
) -> Result<LodesTigerResponse, String> {
    let RegionFilter::Geometry {
        geometry,
        predicate,
    } = region
    else {
        let RegionFilter::Geoids(geoids) = region else {
            unreachable!()
        };
        return run(
            geoids,
            agg_geoid_type,
            wac_segments,
            dataset,
            fallback_policy,
        )
        .await;
    };
    if !matches!(dataset, LodesDataset::WAC { .. }) {
        return Err("polygon LODES queries support WAC datasets only".to_string());
    }
    let target = agg_geoid_type.unwrap_or(GeoidType::Block);
    if matches!(target, GeoidType::Place | GeoidType::CountySubdivision) {
        return Err(format!("LODES cannot aggregate blocks to {target}"));
    }
    let client = Client::new();
    let builder = TigerResourceBuilder::new(dataset.tiger_year())?;
    let resolved =
        spatial_resolver::resolve_polygon(&client, &builder, geometry, target, *predicate).await?;
    let (queries, tiger_errors) = spatial_lodes_queries(dataset, &resolved.matched_geometries);
    if queries.is_empty() {
        return finish_response(Vec::new(), Vec::new(), tiger_errors);
    }
    let agg = agg_geoid_type.map(|g| (g, bamcensus_core::ops::agg::NumericAggregation::Sum));
    let rows =
        lodes_api::run_wac_with_fallback(&client, &queries, wac_segments, agg, fallback_policy)
            .await?;
    let rows = crate::ops::join::filter_to_geometries(rows, &resolved.matched_geometries);
    finish_response(rows, vec![resolved.matched_geometries], tiger_errors)
}

fn spatial_lodes_queries(
    dataset: &LodesDataset,
    geometries: &[(Geoid, Geometry<f64>)],
) -> (Vec<LodesQuery>, Vec<String>) {
    let mut queries = Vec::new();
    let mut errors = Vec::new();
    for state in geometries
        .iter()
        .map(|(geoid, _)| geoid.to_state())
        .unique()
    {
        match dataset.create_query(&state) {
            Ok(query) => queries.push(query),
            Err(error) => errors.push(format!("bamcensus cannot query LODES for {state}: {error}")),
        }
    }
    (queries, errors)
}

fn finish_response(
    lodes_filtered: Vec<(Geoid, Vec<WacValue>)>,
    tiger_rows_nested: Vec<Vec<(Geoid, Geometry<f64>)>>,
    tiger_errors: Vec<String>,
) -> Result<LodesTigerResponse, String> {
    let (join_dataset, join_errors) =
        crate::ops::join::dataset_with_geometries(lodes_filtered, tiger_rows_nested)?;
    let output_dataset = join_dataset
        .into_iter()
        .flat_map(|(geoid, geometry, lodes_values)| {
            lodes_values.into_iter().map(move |lodes_value| {
                LodesWacTigerRow::new(geoid.clone(), lodes_value, geometry.clone())
            })
        })
        .collect_vec();

    let result = LodesTigerResponse {
        join_dataset: output_dataset,
        tiger_errors,
        join_errors,
    };
    Ok(result)
}

fn filter_rows_by_geoids<T>(
    rows: Vec<(Geoid, Vec<T>)>,
    filter_geoids: &[Geoid],
) -> Vec<(Geoid, Vec<T>)> {
    rows.into_iter()
        .filter(|(c, _)| {
            filter_geoids
                .iter()
                .any(|p| p == c || p.is_parent_of(c) || c.is_parent_of(p))
        })
        .collect_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bamcensus_core::model::identifier::{fips, HasGeoidString};
    use bamcensus_core::model::spatial::SpatialPredicate;
    use bamcensus_core::ops::agg::NumericAggregation;
    use bamcensus_lehd::model::WacValue;
    use geo::polygon;

    fn geometry() -> Geometry<f64> {
        Geometry::Polygon(
            polygon![(x: 0.0, y: 0.0), (x: 1.0, y: 0.0), (x: 0.0, y: 1.0), (x: 0.0, y: 0.0)],
        )
    }

    #[test]
    fn polygon_queries_deduplicate_states_and_report_unsupported_territories() {
        let geometries = vec![
            (Geoid::try_from("08031000100").unwrap(), geometry()),
            (Geoid::try_from("08059000100").unwrap(), geometry()),
            (Geoid::try_from("72001000100").unwrap(), geometry()),
        ];
        let (queries, errors) = spatial_lodes_queries(&LodesDataset::default(), &geometries);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].state_code.to_fips_string(), "08");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("72"));
    }

    #[test]
    fn polygon_filter_selects_whole_aggregated_geographies() {
        let value = |n| vec![WacValue::new(WacSegment::C000, n)];
        let rows = vec![
            (Geoid::try_from("080310001001001").unwrap(), value(10.0)),
            (Geoid::try_from("080310001001002").unwrap(), value(20.0)),
            (Geoid::try_from("080310002001001").unwrap(), value(50.0)),
        ];
        let aggregated = bamcensus_lehd::ops::lodes_agg::aggregate_lodes_wac(
            &rows,
            GeoidType::CensusTract,
            NumericAggregation::Sum,
        )
        .unwrap();
        let geometries = vec![(Geoid::try_from("08031000100").unwrap(), geometry())];
        let selected = crate::ops::join::filter_to_geometries(aggregated, &geometries);
        let result = finish_response(selected, vec![geometries], vec![]).unwrap();
        assert_eq!(result.join_dataset.len(), 1);
        assert_eq!(result.join_dataset[0].geoid.geoid_string(), "08031000100");
        assert_eq!(result.join_dataset[0].value.value, 30.0);
        assert!(result.join_errors.is_empty());
    }

    #[tokio::test]
    async fn rejects_unsupported_polygon_lodes_queries_before_download() {
        let region = RegionFilter::new_geometry(geometry(), SpatialPredicate::Intersects).unwrap();
        for target in [GeoidType::Place, GeoidType::CountySubdivision] {
            let result = run_region(
                &region,
                &Some(target),
                &[WacSegment::C000],
                &LodesDataset::default(),
                FallbackPolicy::FailAlways,
            )
            .await;
            assert!(result.err().unwrap().contains("cannot aggregate"));
        }
    }

    #[test]
    fn test_filter_rows_by_geoids_keeps_only_requested_county() {
        let rows = vec![
            (
                Geoid::CensusTract(fips::State(8), fips::County(31), fips::CensusTract(100)),
                vec![WacValue::new(WacSegment::C000, 10.0)],
            ),
            (
                Geoid::CensusTract(fips::State(8), fips::County(59), fips::CensusTract(200)),
                vec![WacValue::new(WacSegment::C000, 20.0)],
            ),
        ];
        let filter_geoids = vec![Geoid::County(fips::State(8), fips::County(31))];
        let filtered = filter_rows_by_geoids(rows, &filter_geoids);
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].0,
            Geoid::CensusTract(fips::State(8), fips::County(31), fips::CensusTract(100))
        );
    }

    #[test]
    fn test_filter_rows_by_geoids_supports_same_level_state_aggregation() {
        let rows = vec![
            (
                Geoid::State(fips::State(8)),
                vec![WacValue::new(WacSegment::C000, 100.0)],
            ),
            (
                Geoid::State(fips::State(6)),
                vec![WacValue::new(WacSegment::C000, 200.0)],
            ),
        ];
        let filter_geoids = vec![Geoid::State(fips::State(8))];
        let filtered = filter_rows_by_geoids(rows, &filter_geoids);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, Geoid::State(fips::State(8)));
    }
}
