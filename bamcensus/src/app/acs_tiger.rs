use crate::model::acs_tiger_row::AcsTigerRow;
use bamcensus_acs::api::acs_api;
use bamcensus_acs::model::{AcsApiQueryParams, AcsGeoidQuery, AcsType, AcsValue};
use bamcensus_core::model::identifier::{Geoid, GeoidType};
use bamcensus_core::model::spatial::RegionFilter;
use bamcensus_tiger::model::TigerResourceBuilder;
use bamcensus_tiger::ops::{spatial_resolver, tiger_api};
use geo::Geometry;
use itertools::Itertools;
use reqwest::Client;

pub struct AcsTigerResponse {
    pub join_dataset: Vec<AcsTigerRow>,
    pub tiger_errors: Vec<String>,
    pub join_errors: Vec<String>,
}

/// Parameters for querying ACS by GEOIDs or a polygonal region.
#[derive(Debug, Clone)]
pub struct AcsRegionQueryParams {
    /// Optional ACS API base URL; defaults to the Census API.
    pub base_url: Option<String>,
    /// ACS and TIGER vintage.
    pub year: u64,
    pub acs_type: AcsType,
    /// ACS variable names.
    pub acs_get_query: Vec<String>,
    pub region: RegionFilter,
    /// Required for polygon queries. For GEOIDs this is the ACS wildcard level.
    pub output_resolution: Option<GeoidType>,
    pub acs_token: Option<String>,
}

/// runs a query to ACS. the result will include a list of GEOIDs alongside
/// ACS data. all GEOIDs are used to run a set of downloads from the TIGER/Lines
/// datasets. the geometries from TIGER are combined with the ACS data producing
/// AcsTigerRows.
///
/// # Example
///
/// ```no_run
/// use bamcensus::app::acs_tiger;
/// use bamcensus_acs::model::{AcsApiQueryParams, AcsGeoidQuery, AcsType};
/// use bamcensus_core::model::identifier::{fips, Geoid, GeoidType};
///
/// let year = 2020;
/// let acs_type = AcsType::FiveYear;
/// let acs_get_query = vec![String::from("NAME"), String::from("B01001_001E")];
/// let geoid = Geoid::State(fips::State(8));
/// let wildcard = Some(GeoidType::County);
///
/// let acs_geoid_query = AcsGeoidQuery::new(Some(geoid), wildcard).unwrap();
/// let query_params = AcsApiQueryParams::new(
///     None,
///     year,
///     acs_type,
///     acs_get_query,
///     acs_geoid_query,
///     None,
/// );
/// # tokio_test::block_on(async {
/// let res = acs_tiger::run(&query_params).await.unwrap();
/// println!(
///     "found {} responses, {}/{} errors",
///     res.join_dataset.len(),
///     res.tiger_errors.len(),
///     res.join_errors.len(),
/// );
/// for row in res.join_dataset.into_iter() {
///     println!("{}", row)
/// }
/// # })
/// ```
pub async fn run(query: &AcsApiQueryParams) -> Result<AcsTigerResponse, String> {
    run_batch(&vec![query.clone()]).await
}

pub async fn run_batch(queries: &Vec<AcsApiQueryParams>) -> Result<AcsTigerResponse, String> {
    let client: Client = Client::new();

    // todo: run tiger downloads for all requested years
    let year = match &queries.iter().map(|q| q.year).unique().collect_vec()[..] {
        [one_year] => Ok(*one_year),
        years => Err(format!(
            "acs.run_batch with queries should be run with one matching year for optimal geometry downloads, but found the following years: [{}]",
            years.iter().map(|y| format!("{y}")).join(",")
        )),
    }?;

    let acs_rows = acs_api::batch_run(&client, queries).await?;

    // execute TIGER/Lines downloads
    let tiger_uri_builder = TigerResourceBuilder::new(year)?;
    let geoids = &acs_rows.iter().map(|(geoid, _)| geoid).collect_vec();
    let tiger_response = tiger_api::run(&client, &tiger_uri_builder, geoids).await?;

    type NestedResult = (Vec<Vec<(Geoid, Geometry<f64>)>>, Vec<String>);
    let (tiger_rows_nested, tiger_errors): NestedResult =
        tiger_response.into_iter().partition_result();

    finish_response(acs_rows, tiger_rows_nested, tiger_errors)
}

/// Query ACS for an explicit GEOID selection or for whole geographies matching
/// a polygon. Polygon queries require an output resolution other than block.
///
/// The returned values cover each selected geography in full, not only the
/// portion inside the polygon. TIGER spatial calculations are planar; see
/// [`spatial_resolver::resolve_polygon`] for their CRS and geometry requirements.
pub async fn run_region(params: &AcsRegionQueryParams) -> Result<AcsTigerResponse, String> {
    match &params.region {
        RegionFilter::Geoids(geoids) => {
            let queries = if geoids.is_empty() {
                vec![AcsGeoidQuery::new(None, params.output_resolution)?]
            } else {
                geoids
                    .iter()
                    .map(|geoid| AcsGeoidQuery::new(Some(geoid.clone()), params.output_resolution))
                    .collect::<Result<Vec<_>, _>>()?
            };
            run_batch(&acs_query_params(params, queries)).await
        }
        RegionFilter::Geometry {
            geometry,
            predicate,
        } => {
            let target = params
                .output_resolution
                .ok_or_else(|| "polygon ACS queries require an output resolution".to_string())?;
            if target == GeoidType::Block {
                return Err("ACS does not support block-level queries".to_string());
            }
            let client = Client::new();
            let builder = TigerResourceBuilder::new(params.year)?;
            let resolved =
                spatial_resolver::resolve_polygon(&client, &builder, geometry, target, *predicate)
                    .await?;
            if resolved.matched_geometries.is_empty() {
                return finish_response(Vec::new(), Vec::new(), Vec::new());
            }
            let queries = spatial_acs_queries(target, &resolved.matched_geometries)?;
            let rows = acs_api::batch_run(&client, &acs_query_params(params, queries)).await?;
            let rows = crate::ops::join::filter_to_geometries(rows, &resolved.matched_geometries);
            finish_response(rows, vec![resolved.matched_geometries], Vec::new())
        }
    }
}

fn acs_query_params(
    params: &AcsRegionQueryParams,
    queries: Vec<AcsGeoidQuery>,
) -> Vec<AcsApiQueryParams> {
    queries
        .into_iter()
        .map(|query| {
            AcsApiQueryParams::new(
                params.base_url.clone(),
                params.year,
                params.acs_type,
                params.acs_get_query.clone(),
                query,
                params.acs_token.clone(),
            )
        })
        .collect()
}

fn spatial_acs_queries(
    target: GeoidType,
    geometries: &[(Geoid, Geometry<f64>)],
) -> Result<Vec<AcsGeoidQuery>, String> {
    let parents = geometries
        .iter()
        .map(|(geoid, _)| {
            if geoid.geoid_type() != target {
                return Err(format!(
                    "expected {target} geometry, found {}",
                    geoid.geoid_type()
                ));
            }
            match target {
                GeoidType::State => Ok(geoid.clone()),
                GeoidType::County | GeoidType::Place => Ok(geoid.to_state()),
                GeoidType::CountySubdivision | GeoidType::CensusTract | GeoidType::BlockGroup => {
                    geoid.to_county()
                }
                GeoidType::Block => Err("ACS does not support block-level queries".to_string()),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    parents
        .into_iter()
        .unique()
        .map(|parent| {
            let wildcard = if target == GeoidType::State {
                None
            } else {
                Some(target)
            };
            AcsGeoidQuery::new(Some(parent), wildcard)
        })
        .collect()
}

fn finish_response(
    acs_rows: Vec<(Geoid, Vec<AcsValue>)>,
    tiger_rows_nested: Vec<Vec<(Geoid, Geometry<f64>)>>,
    tiger_errors: Vec<String>,
) -> Result<AcsTigerResponse, String> {
    let (join_dataset, join_errors) =
        crate::ops::join::dataset_with_geometries(acs_rows, tiger_rows_nested)?;
    let output_dataset = join_dataset
        .into_iter()
        .flat_map(|(geoid, geometry, acs_values)| {
            acs_values
                .into_iter()
                .map(move |acs_value| AcsTigerRow::new(geoid.clone(), acs_value, geometry.clone()))
        })
        .collect_vec();

    let result = AcsTigerResponse {
        join_dataset: output_dataset,
        tiger_errors,
        join_errors,
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bamcensus_core::model::spatial::SpatialPredicate;
    use geo::{polygon, Geometry};

    fn geometries(ids: &[&str]) -> Vec<(Geoid, Geometry<f64>)> {
        ids.iter()
            .map(|id| {
                (
                    Geoid::try_from(*id).unwrap(),
                    Geometry::Polygon(polygon![(x: 0.0, y: 0.0), (x: 1.0, y: 0.0), (x: 0.0, y: 1.0), (x: 0.0, y: 0.0)]),
                )
            })
            .collect()
    }

    #[test]
    fn spatial_queries_use_appropriate_parents_without_duplicates() {
        let cases = [
            (GeoidType::State, vec!["08"], vec!["&for=state:08"]),
            (
                GeoidType::County,
                vec!["08031", "08059"],
                vec!["&for=county:*&in=state:08"],
            ),
            (
                GeoidType::Place,
                vec!["0812345", "0854321"],
                vec!["&for=place:*&in=state:08"],
            ),
            (
                GeoidType::CountySubdivision,
                vec!["0803112345"],
                vec!["&for=county%20subdivision:*&in=state:08&in=county:031"],
            ),
            (
                GeoidType::CensusTract,
                vec!["08031000100", "08031000200", "08059000100"],
                vec![
                    "&for=tract:*&in=state:08&in=county:031",
                    "&for=tract:*&in=state:08&in=county:059",
                ],
            ),
            (
                GeoidType::BlockGroup,
                vec!["080310001001"],
                vec!["&for=block%20group:*&in=state:08&in=county:031&in=tract:*"],
            ),
        ];
        for (target, ids, expected) in cases {
            let queries = spatial_acs_queries(target, &geometries(&ids)).unwrap();
            assert_eq!(
                queries
                    .iter()
                    .map(AcsGeoidQuery::to_query_key)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn filtered_acs_values_join_only_matching_geometries() {
        let geometry = geometries(&["08031000100"]);
        let value = |name: &str, value: &str| AcsValue::new(name.to_string(), value.into());
        let rows = vec![
            (
                Geoid::try_from("08031000100").unwrap(),
                vec![value("NAME", "selected"), value("B01001_001E", "10")],
            ),
            (
                Geoid::try_from("08031000200").unwrap(),
                vec![value("NAME", "excluded")],
            ),
        ];
        let rows = crate::ops::join::filter_to_geometries(rows, &geometry);
        let result = finish_response(rows, vec![geometry], vec![]).unwrap();
        assert_eq!(result.join_dataset.len(), 2);
        assert!(result
            .join_dataset
            .iter()
            .all(|row| row.geoid == Geoid::try_from("08031000100").unwrap()));
        assert_eq!(result.join_dataset[0].acs_value.value, "selected");
        assert_eq!(result.join_dataset[1].acs_value.value, "10");
        assert!(result.join_errors.is_empty());
        assert!(result.tiger_errors.is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_polygon_acs_resolution_before_download() {
        let region = RegionFilter::new_geometry(
            geometries(&["08"]).remove(0).1,
            SpatialPredicate::Intersects,
        )
        .unwrap();
        let mut params = AcsRegionQueryParams {
            base_url: None,
            year: 2020,
            acs_type: AcsType::FiveYear,
            acs_get_query: vec!["NAME".to_string()],
            region,
            output_resolution: None,
            acs_token: None,
        };
        assert!(run_region(&params)
            .await
            .err()
            .unwrap()
            .contains("require an output resolution"));
        params.output_resolution = Some(GeoidType::Block);
        assert!(run_region(&params).await.err().unwrap().contains("block"));
    }
}
