use crate::model::{FallbackPolicy, WacRow, WacSegment, WacValue};
use crate::ops::lodes_agg;
use bamcensus_core::{
    model::identifier::{Geoid, GeoidType},
    ops::agg::NumericAggregation,
};
use csv::ReaderBuilder;
use flate2::read::GzDecoder;
use futures::future;
use itertools::Itertools;
use kdam::BarExt;
use reqwest::Client;
use std::sync::{Arc, Mutex};

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
        return Err(format!(
            "LODES HTTP error {}: {}",
            status.as_u16(),
            url
        ));
    }

    let gzip_bytes = res
        .bytes()
        .await
        .map_err(|e| format!("failure reading response body: {e}"))?;

    let mut reader = ReaderBuilder::new().from_reader(GzDecoder::new(&gzip_bytes[..]));
    let mut result = vec![];
    for r in reader.deserialize() {
        let row: WacRow =
            r.map_err(|e| format!("failure reading LODES response row: {e}"))?;
        let geoid = row.geoid()?;
        let mut row_result = vec![];
        for segment in wac_segments.iter() {
            row_result.push(WacValue::new(*segment, row.get(segment)));
        }
        result.push((geoid, row_result));
    }

    Ok(Some(result))
}

/// Applies the fallback policy to a LODES URL query.
///
/// Given a base URL that may contain `{year}` placeholder, generates alternative URLs
/// based on the fallback policy and attempts each in sequence until one succeeds.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests
/// * `base_url` - URL with `{year}` placeholder (e.g., `.../2021/...` where `2021` is the year)
/// * `base_year` - The originally requested year
/// * `fallback_policy` - Policy for retry attempts
/// * `wac_segments` - WAC segments to extract from the response
///
/// # Returns
///
/// - `Ok(Some(Vec<(Geoid, Vec<WacValue>)>))` if any URL succeeds
/// - `Ok(None)` if all URLs return 404
/// - `Err(String)` if a fatal error occurs
async fn run_with_fallback(
    client: &Client,
    base_url: &str,
    base_year: u64,
    fallback_policy: FallbackPolicy,
    wac_segments: &[WacSegment],
) -> Result<Option<Vec<(Geoid, Vec<WacValue>)>>, String> {
    let candidate_years = fallback_policy.candidate_years(base_year);

    for year in candidate_years {
        let url = base_url.replace(&base_year.to_string(), &year.to_string());
        match try_fetch_lodes_data(client, &url, wac_segments).await {
            Ok(Some(data)) => {
                if year != base_year {
                    eprintln!(
                        "LODES data for year {} not available; using {} instead",
                        base_year, year
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
    queries: &[String],
    wac_segments: &[WacSegment],
    agg: Option<(GeoidType, NumericAggregation)>,
) -> Result<Vec<(Geoid, Vec<WacValue>)>, String> {
    run_wac_with_fallback(client, queries, wac_segments, agg, FallbackPolicy::FailAlways).await
}

/// runs a set of LODES queries with support for fallback policy when state/year is unavailable.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests
/// * `queries` - List of LODES URLs to fetch
/// * `wac_segments` - WAC segments to extract from each response
/// * `agg` - Optional aggregation specification
/// * `fallback_policy` - Policy for handling unavailable state/year combinations
pub async fn run_wac_with_fallback(
    client: &Client,
    queries: &[String],
    wac_segments: &[WacSegment],
    agg: Option<(GeoidType, NumericAggregation)>,
    fallback_policy: FallbackPolicy,
) -> Result<Vec<(Geoid, Vec<WacValue>)>, String> {
    // setup progress bar
    let pb_builder = kdam::BarBuilder::default()
        .total(queries.len())
        .desc("LODES downloads");
    let pb = Arc::new(Mutex::new(pb_builder.build()?));

    // Extract base year from the first query URL for fallback policy
    // URLs typically follow pattern: .../LODES/public/lodes8/2021/RI_wac_S000_JT00_2021.csv.gz
    let extract_year = |url: &str| -> Option<u64> {
        url.split('/').find_map(|part| part.parse::<u64>().ok())
    };

    let base_year = queries
        .first()
        .and_then(|url| extract_year(url))
        .unwrap_or_else(|| {
            eprintln!("warning: could not extract year from LODES URLs");
            2021
        });

    // run each query in parallel with fallback logic
    let responses = queries.iter().map(|url| {
        let client = &client;
        let wac_segments = &wac_segments;
        let pb = pb.clone();
        async move {
            let result = run_with_fallback(client, url, base_year, fallback_policy, wac_segments).await;

            // update progress bar
            let _pb_update_result = pb
                .lock()
                .map_err(|e| format!("failure aquiring progress bar mutex lock: {e}"))
                .and_then(|mut pb_guard| {
                    pb_guard
                        .update(1)
                        .map_err(|e| format!("failure on pb update: {e}"))
                })?;

            result.map(|opt| opt.unwrap_or_default())
        }
    });
    eprintln!(); // progress bar terminated

    // join query result
    let response_rows = future::join_all(responses)
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
