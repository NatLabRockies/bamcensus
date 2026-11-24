use crate::model::{FallbackPolicy, LodesQuery, WacRow, WacSegment, WacValue};
use crate::ops::lodes_agg;
use bamcensus_core::{
    model::identifier::{Geoid, GeoidType},
    ops::agg::NumericAggregation,
};
use csv::ReaderBuilder;
use flate2::read::GzDecoder;
use futures::{stream, StreamExt};
use kdam::BarExt;
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

const MAX_CONCURRENT_DOWNLOADS: usize = 8;

/// Downloaded WAC rows and the successful source year keyed by two-digit state FIPS.
/// States skipped after exhausting fallback candidates are omitted.
#[derive(Serialize, Deserialize)]
pub struct LodesWacResponse {
    pub rows: Vec<(Geoid, Vec<WacValue>)>,
    pub selected_years: BTreeMap<String, u64>,
}

/// Attempts to fetch and parse a single LODES dataset from a URL.
///
/// # Returns
///
/// - `Ok(Some(Vec<(Geoid, Vec<WacValue>)>))` if the URL returns data
/// - `Ok(None)` if the URL returns a 404 (not found, will trigger fallback)
/// - `Err(String)` for other errors that should not trigger fallback
async fn try_fetch_lodes_data(
    client: &ClientWithMiddleware,
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
/// - `Ok(Some((year, rows)))` if any candidate query succeeds
/// - `Ok(None)` if all candidate queries are unavailable or return 404
/// - `Err(String)` if a fatal error occurs
async fn run_with_fallback(
    client: &ClientWithMiddleware,
    query: &LodesQuery,
    fallback_policy: FallbackPolicy,
    wac_segments: &[WacSegment],
) -> Result<Option<(u64, Vec<(Geoid, Vec<WacValue>)>)>, String> {
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
                    eprintln!(
                        "LODES data for year {} not available; using {} instead",
                        base_year, year
                    );
                }
                return Ok(Some((year, data)));
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
    client: &ClientWithMiddleware,
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
    client: &ClientWithMiddleware,
    queries: &[LodesQuery],
    wac_segments: &[WacSegment],
    agg: Option<(GeoidType, NumericAggregation)>,
    fallback_policy: FallbackPolicy,
) -> Result<Vec<(Geoid, Vec<WacValue>)>, String> {
    Ok(
        run_wac_queries(client, queries, wac_segments, agg, fallback_policy, false)
            .await?
            .rows,
    )
}

/// Run WAC queries while preserving the successful year for each state.
/// Aggregation changes the rows but does not change their source-year metadata.
/// Returns an error if queries select multiple source years for the same state.
pub async fn run_wac_with_fallback_and_metadata(
    client: &ClientWithMiddleware,
    queries: &[LodesQuery],
    wac_segments: &[WacSegment],
    agg: Option<(GeoidType, NumericAggregation)>,
    fallback_policy: FallbackPolicy,
) -> Result<LodesWacResponse, String> {
    run_wac_queries(client, queries, wac_segments, agg, fallback_policy, true).await
}

async fn run_wac_queries(
    client: &ClientWithMiddleware,
    queries: &[LodesQuery],
    wac_segments: &[WacSegment],
    agg: Option<(GeoidType, NumericAggregation)>,
    fallback_policy: FallbackPolicy,
    preserve_years: bool,
) -> Result<LodesWacResponse, String> {
    // setup progress bar
    let pb_builder = kdam::BarBuilder::default()
        .total(queries.len())
        .desc("LODES downloads");
    let pb = Arc::new(Mutex::new(pb_builder.build()?));

    // run each query in parallel with bounded concurrency and fallback logic
    let responses = queries.iter().map(|query| {
        let client = &client;
        let wac_segments = &wac_segments;
        let pb = pb.clone();
        async move {
            let result = run_with_fallback(client, query, fallback_policy, wac_segments).await;

            // update progress bar
            let _pb_update_result = pb
                .lock()
                .map_err(|e| format!("failure aquiring progress bar mutex lock: {e}"))
                .and_then(|mut pb_guard| {
                    pb_guard
                        .update(1)
                        .map_err(|e| format!("failure on pb update: {e}"))
                })?;

            match result? {
                Some((year, data)) => Ok((Some((query.state_code.to_fips_string(), year)), data)),
                None if fallback_policy == FallbackPolicy::FailAlways => {
                    Err(format!("LODES data not available for {query}"))
                }
                None => {
                    eprintln!("LODES data not available for {query} after exhausting fallback years; skipping");
                    Ok((None, Vec::new()))
                }
            }
        }
    });

    // join query result
    let responses = stream::iter(responses)
        .buffer_unordered(MAX_CONCURRENT_DOWNLOADS)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, String>>()?;
    let mut selected_years = BTreeMap::new();
    let mut response_rows = Vec::new();
    for (selected_year, rows) in responses {
        if preserve_years {
            if let Some((state, year)) = selected_year {
                if let Some(previous) = selected_years.insert(state.clone(), year) {
                    if previous != year {
                        return Err(format!("multiple LODES source years selected for state {state}: {previous} and {year}"));
                    }
                }
            }
        }
        response_rows.extend(rows);
    }
    eprintln!(); // progress bar terminated

    // if requested, aggregate the result
    let aggregated_rows = match agg {
        Some((output_geoid_type, agg)) => {
            lodes_agg::aggregate_lodes_wac(&response_rows, output_geoid_type, agg)?
        }
        None => response_rows.to_vec(),
    };
    Ok(LodesWacResponse {
        rows: aggregated_rows,
        selected_years,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LodesDataset;
    use clap::ValueEnum;
    use flate2::{write::GzEncoder, Compression};
    use reqwest_middleware::{ClientBuilder, RequestBuilder};
    use std::io::Write;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    fn gzip_rows(state: &str) -> Vec<u8> {
        let segments = WacSegment::value_variants();
        let mut writer = csv::Writer::from_writer(Vec::new());
        let mut header = vec!["w_geocode".to_string()];
        header.extend(segments.iter().map(ToString::to_string));
        header.push("createdate".to_string());
        writer.write_record(header).unwrap();
        if state != "46" {
            for block in ["001", "002"] {
                let mut row = vec![format!("{state}0010001001{block}")];
                row.extend(segments.iter().map(|_| "10".to_string()));
                row.push("20200101".to_string());
                writer.write_record(row).unwrap();
            }
        }
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&writer.into_inner().unwrap()).unwrap();
        gzip.finish().unwrap()
    }

    async fn mock_client() -> (ClientWithMiddleware, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let client = ClientBuilder::new(reqwest_middleware::reqwest::Client::new())
            .with_init(move |request: RequestBuilder| {
                let (client, request) = request.build_split();
                let mut request = request.unwrap();
                *request.url_mut() = format!("{base}{}", request.url().path()).parse().unwrap();
                RequestBuilder::from_parts(client, request)
            })
            .build();
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut reader = BufReader::new(&mut socket);
                let mut request = String::new();
                reader.read_line(&mut request).await.unwrap();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap() == 0 || line == "\r\n" {
                        break;
                    }
                }
                let (status, body) = if request.contains("co_wac_S000_JT00_2020")
                    || request.contains("co_wac_S000_JT00_2021")
                {
                    (200, gzip_rows("08"))
                } else if request.contains("ca_wac_S000_JT00_2019") {
                    (200, gzip_rows("06"))
                } else if request.contains("ak_wac_S000_JT00_2016") {
                    (200, gzip_rows("02"))
                } else if request.contains("sd_wac_S000_JT00_2020") {
                    (200, gzip_rows("46"))
                } else if request.contains("ri_wac_S000_JT00_2021") {
                    (503, Vec::new())
                } else if request.contains("nv_wac_S000_JT00_2020") {
                    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
                    gzip.write_all(b"w_geocode,C000\n080010001001001,invalid\n")
                        .unwrap();
                    (200, gzip.finish().unwrap())
                } else {
                    (404, Vec::new())
                };
                let headers = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                socket.write_all(headers.as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            }
        });
        (client, server)
    }

    fn query(state: &str, year: u64) -> LodesQuery {
        LodesDataset::default()
            .with_year(year)
            .create_query(&Geoid::try_from(state).unwrap())
            .unwrap()
    }

    #[tokio::test]
    async fn selected_years_survive_fallback_and_aggregation_and_omit_skipped_states() {
        let (client, server) = mock_client().await;
        let queries = ["08", "06", "02", "44", "46"].map(|state| query(state, 2020));
        let result = run_wac_with_fallback_and_metadata(
            &client,
            &queries,
            &[WacSegment::C000],
            Some((GeoidType::County, NumericAggregation::Sum)),
            FallbackPolicy::TryPreviousYears(4),
        )
        .await
        .unwrap();
        assert_eq!(
            result.selected_years,
            BTreeMap::from([
                ("02".to_string(), 2016),
                ("06".to_string(), 2019),
                ("08".to_string(), 2020),
                ("46".to_string(), 2020),
            ])
        );
        assert_eq!(result.rows.len(), 3);
        assert!(result
            .rows
            .iter()
            .all(|(_, values)| values[0].value == 20.0));
        server.abort();
    }

    #[tokio::test]
    async fn selected_years_support_forward_fallback_and_rows_only_api() {
        let (client, server) = mock_client().await;
        let result = run_wac_with_fallback_and_metadata(
            &client,
            &[query("08", 2019)],
            &[WacSegment::C000],
            None,
            FallbackPolicy::TryNextYears(1),
        )
        .await
        .unwrap();
        assert_eq!(result.selected_years["08"], 2020);
        assert_eq!(result.rows.len(), 2);
        let rows = run_wac(&client, &[query("08", 2020)], &[WacSegment::C000], None)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        let error = run_wac(&client, &[query("08", 2019)], &[WacSegment::C000], None)
            .await
            .err()
            .unwrap();
        assert!(error.contains("not available"));
        server.abort();
    }

    #[tokio::test]
    async fn metadata_does_not_hide_http_or_parse_errors() {
        let (client, server) = mock_client().await;
        for (state, year, message) in [("44", 2021, "503"), ("32", 2020, "failure reading")] {
            let error = run_wac_with_fallback_and_metadata(
                &client,
                &[query(state, year)],
                &[WacSegment::C000],
                None,
                FallbackPolicy::TryPreviousYears(2),
            )
            .await
            .err()
            .unwrap();
            assert!(error.contains(message), "{error}");
        }
        server.abort();
    }

    #[tokio::test]
    async fn conflicting_state_years_are_rejected_only_by_metadata_api() {
        let (client, server) = mock_client().await;
        let queries = [query("06", 2019), query("06", 2020)];
        let result = run_wac_with_fallback_and_metadata(
            &client,
            &queries,
            &[WacSegment::C000],
            None,
            FallbackPolicy::TryPreviousYears(1),
        )
        .await
        .unwrap();
        assert_eq!(result.selected_years["06"], 2019);
        assert_eq!(result.rows.len(), 4);

        let queries = [query("08", 2020), query("08", 2021)];
        let error = run_wac_with_fallback_and_metadata(
            &client,
            &queries,
            &[WacSegment::C000],
            None,
            FallbackPolicy::FailAlways,
        )
        .await
        .err()
        .unwrap();
        assert!(error.contains("multiple LODES source years"));
        let rows = run_wac(&client, &queries, &[WacSegment::C000], None)
            .await
            .unwrap();
        assert_eq!(rows.len(), 4);
        server.abort();
    }
}
