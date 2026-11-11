use crate::model::lodes_wac_tiger_row::LodesWacTigerRow;
use bamcensus_core::model::identifier::Geoid;
use bamcensus_core::model::identifier::GeoidType;
use bamcensus_lehd::api::lodes_api;
use bamcensus_lehd::model::{LodesDataset, FallbackPolicy, WacSegment};
use bamcensus_tiger::model::TigerResourceBuilder;
use bamcensus_tiger::ops::tiger_api;
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
/// ```ignore
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
    let geoids = match geoids.len() {
        0 => Geoid::all_states(),
        _ => geoids.iter().map(|g| g.to_state()).unique().collect_vec(),
    };
    let lodes_queries = geoids
        .iter()
        .map(|geoid| dataset.create_uri(geoid))
        .collect::<Result<Vec<_>, _>>()?;

    let agg_fn = bamcensus_core::ops::agg::NumericAggregation::Sum;
    let agg = agg_geoid_type.map(|g| (g, agg_fn));

    // execute LODES downloads with fallback policy

    let client: Client = Client::new();
    let lodes_rows = lodes_api::run_wac_with_fallback(&client, &lodes_queries, wac_segments, agg, fallback_policy).await?;

    // filter result. LODES collects by State. here we only accept rows where the
    // input geoids are the (FIPS hierarchical) parent.
    let lodes_filtered = lodes_rows
        .into_iter()
        .filter(|(c, _)| geoids.iter().any(|p| p.is_parent_of(c)))
        .collect_vec();

    // execute TIGER/Lines downloads selecting a data vintage based on the LODES edition chosen
    let tiger_year = dataset.tiger_year();
    let tiger_uri_builder = TigerResourceBuilder::new(tiger_year)?;
    let lodes_geoids = &lodes_filtered.iter().map(|(geoid, _)| geoid).collect_vec();
    let tiger_response = tiger_api::run(&client, &tiger_uri_builder, lodes_geoids).await?;

    type NestedResult = (Vec<Vec<(Geoid, Geometry<f64>)>>, Vec<String>);
    let (tiger_rows_nested, tiger_errors): NestedResult =
        tiger_response.into_iter().partition_result();

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
