use crate::model::{FallbackPolicy, LodesQuery, WacRow, WacSegment, WacValue};
use crate::ops::lodes_agg;
use bamcensus_core::{
    model::identifier::{Geoid, GeoidType},
    ops::agg::NumericAggregation,
};
use csv::ReaderBuilder;
use flate2::read::GzDecoder;
use futures::{stream, StreamExt};
use itertools::Itertools;
use reqwest::Client;

const MAX_CONCURRENT_DOWNLOADS: usize = 8;

/// Attempts to fetch and parse a single LODES dataset from a URL.
///
/// # Returns
///
/// - `Ok(Some(Vec<(Geoid, Vec<WacValue>)>))` if the URL returns data
/// - `Ok(None)` if the URL returns a 404 (not found, will trigger fallback)
/// - `Err(String)` for other errors that should not trigger fallback
async fn try_fetch_lodes_data(
    client: &Client,
    url: &str,
    wac_segments: &[WacSegment],
) -> Result<Option<Vec<(Geoid, Vec<WacValue>)>>, String> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("failure sending LODES HTTP request: {e}"))?;

    // Handle 404 as a recoverable error (state/year not available)
    if res.status() == 404 {
        return Ok(None);
    }

    // Fail on other HTTP errors
    let status = res.status();
    if !status.is_success() {
        return Err(format!("LODES HTTP error {}: {}", status.as_u16(), url));
    }

    let gzip_bytes = res
        .bytes()
        .await
        .map_err(|e| format!("failure reading response body: {e}"))?;

    let mut reader = ReaderBuilder::new().from_reader(GzDecoder::new(&gzip_bytes[..]));
    let mut result = vec![];
    for r in reader.deserialize() {
        let row: WacRow = r.map_err(|e| format!("failure reading LODES response row: {e}"))?;
        let geoid = row.geoid()?;
        let mut row_result = vec![];
        for segment in wac_segments.iter() {
            row_result.push(WacValue::new(*segment, row.get(segment)));
        }
        result.push((geoid, row_result));
    }

    Ok(Some(result))
}

/// Applies the fallback policy to a structured LODES query.
///
/// Generates candidate years based on the fallback policy, materializes each candidate query
/// into a URL, and attempts each in sequence until one succeeds.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests
/// * `query` - Structured LODES query with dataset parameters and state
/// * `fallback_policy` - Policy for retry attempts
/// * `wac_segments` - WAC segments to extract from the response
///
/// # Returns
///
/// - `Ok(Some(Vec<(Geoid, Vec<WacValue>)>))` if any candidate query succeeds
/// - `Ok(None)` if all candidate queries are unavailable or return 404
/// - `Err(String)` if a fatal error occurs
async fn run_with_fallback(
    client: &Client,
    query: &LodesQuery,
    fallback_policy: FallbackPolicy,
    wac_segments: &[WacSegment],
) -> Result<Option<Vec<(Geoid, Vec<WacValue>)>>, String> {
    let base_year = query.year();
    let candidate_years = fallback_policy.candidate_years(base_year);

    for year in candidate_years {
        let candidate_query = query.with_year(year);
        let url = match candidate_query.to_uri() {
            Ok(url) => url,
            Err(_) => {
                // Statically known unavailable state/year combination, continue to next fallback candidate
                continue;
            }
        };
        match try_fetch_lodes_data(client, &url, wac_segments).await {
            Ok(Some(data)) => {
                if year != base_year {
                    log::warn!(
                        "LODES data for year {} not available; using {} instead",
                        base_year,
                        year
                    );
                }
                return Ok(Some(data));
            }
            Ok(None) => {
                // 404, continue to next fallback candidate
                continue;
            }
            Err(e) => {
                // Fatal error, don't retry
                return Err(e);
            }
        }
    }

    // All candidates exhausted without success
    Ok(None)
}

/// runs a set of LODES queries. each required LODES file is collected in
/// memory and deserialized into rows of Geoids with WacValues for each
/// requested WacSegment. the entire dataset is aggregated to the requested
/// output GeoidType, which should be
pub async fn run_wac(
    client: &Client,
    queries: &[LodesQuery],
    wac_segments: &[WacSegment],
    agg: Option<(GeoidType, NumericAggregation)>,
) -> Result<Vec<(Geoid, Vec<WacValue>)>, String> {
    run_wac_with_fallback(
        client,
        queries,
        wac_segments,
        agg,
        FallbackPolicy::FailAlways,
    )
    .await
}

/// runs a set of LODES queries with support for fallback policy when state/year is unavailable.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests
/// * `queries` - List of structured LODES queries to fetch
/// * `wac_segments` - WAC segments to extract from each response
/// * `agg` - Optional aggregation specification
/// * `fallback_policy` - Policy for handling unavailable state/year combinations
pub async fn run_wac_with_fallback(
    client: &Client,
    queries: &[LodesQuery],
    wac_segments: &[WacSegment],
    agg: Option<(GeoidType, NumericAggregation)>,
    fallback_policy: FallbackPolicy,
) -> Result<Vec<(Geoid, Vec<WacValue>)>, String> {
    // run each query in parallel with bounded concurrency and fallback logic
    let responses = queries.iter().map(|query| {
        let client = &client;
        let wac_segments = &wac_segments;
        async move {
            let result = run_with_fallback(client, query, fallback_policy, wac_segments).await;

            match result? {
                Some(data) => Ok(data),
                None if fallback_policy == FallbackPolicy::FailAlways => {
                    Err(format!("LODES data not available for {query}"))
                }
                None => Ok(Vec::new()),
            }
        }
    });

    // join query result
    let response_rows = stream::iter(responses)
        .buffer_unordered(MAX_CONCURRENT_DOWNLOADS)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect_vec();

    // if requested, aggregate the result
    let aggregated_rows = match agg {
        Some((output_geoid_type, agg)) => {
            lodes_agg::aggregate_lodes_wac(&response_rows, output_geoid_type, agg)?
        }
        None => response_rows.to_vec(),
    };
    Ok(aggregated_rows)
}
