use bamcensus::app::{acs_tiger, lodes_tiger};
use bamcensus_acs::model::AcsType;
use bamcensus_core::model::identifier::{Geoid, GeoidType, HasGeoidString};
use bamcensus_core::model::spatial::{RegionFilter, SpatialPredicate};
use bamcensus_lehd::model::{FallbackPolicy, LodesDataset, WacSegment};
use geo::{polygon, Geometry, Polygon};
use shapefile::dbase::{FieldValue, Record, TableWriterBuilder};
use std::{fs, io::Write, path::Path, process::Command, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use zip::{write::SimpleFileOptions, ZipWriter};

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon<f64> {
    polygon![(x: x0, y: y0), (x: x1, y: y0), (x: x1, y: y1), (x: x0, y: y1), (x: x0, y: y0)]
}

fn region(polygon: Polygon<f64>, predicate: SpatialPredicate) -> RegionFilter {
    RegionFilter::new_geometry(Geometry::Polygon(polygon), predicate).unwrap()
}

fn write_archive(root: &Path, relative_path: &str, records: &[(&str, Polygon<f64>)]) {
    let path = root.join("fixture.shp");
    let fields = TableWriterBuilder::new().add_character_field("GEOID".try_into().unwrap(), 16);
    {
        let mut writer = shapefile::Writer::from_path(&path, fields).unwrap();
        for (geoid, polygon) in records {
            let mut record = Record::default();
            record.insert(
                "GEOID".to_string(),
                FieldValue::Character(Some(geoid.to_string())),
            );
            writer
                .write_shape_and_record(&shapefile::Polygon::from(polygon.clone()), &record)
                .unwrap();
        }
    }
    let archive_path = root.join("tiger").join(relative_path);
    fs::create_dir_all(archive_path.parent().unwrap()).unwrap();
    let mut zip = ZipWriter::new(fs::File::create(archive_path).unwrap());
    for extension in ["shp", "dbf"] {
        zip.start_file(format!("fixture.{extension}"), SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&fs::read(path.with_extension(extension)).unwrap())
            .unwrap();
    }
    zip.finish().unwrap();
}

fn run_child(test_name: &str, child_env: &str, cache: &Path, ignored: bool) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    if ignored {
        command.arg("--ignored");
    }
    let output = command
        .args(["--exact", test_name, "--nocapture"])
        .env(child_env, "1")
        .env("BAMCENSUS_CACHE_DIR", cache)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn polygon_acs_joins_cached_geometries_and_filters_wildcard_results() {
    const CHILD: &str = "BAMCENSUS_ACS_REGION_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let mut requests = Vec::new();
                for _ in 0..2 {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let mut reader = BufReader::new(&mut socket);
                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    requests.push(line.clone());
                    loop {
                        line.clear();
                        assert!(reader.read_line(&mut line).await.unwrap() > 0);
                        if line == "\r\n" { break; }
                    }
                    let header = r#"[["NAME","B01001_001E","state","county","tract"],["selected","10","08","031","000100"]"#;
                    let body = if requests.last().unwrap().contains("tract:*") {
                        format!(r#"{header},["excluded","20","08","031","000200"]]"#)
                    } else {
                        format!("{header}]")
                    };
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    socket.write_all(response.as_bytes()).await.unwrap();
                }
                requests
            });
            let mut params = acs_tiger::AcsRegionQueryParams {
                base_url: Some(base_url),
                year: 2020,
                acs_type: AcsType::FiveYear,
                acs_get_query: vec!["NAME".into(), "B01001_001E".into()],
                region: region(square(1.0, 1.0, 4.0, 4.0), SpatialPredicate::Within),
                output_resolution: Some(GeoidType::CensusTract),
                acs_token: None,
            };
            let spatial = acs_tiger::run_region(&params).await.unwrap();
            assert!(spatial.tiger_errors.is_empty() && spatial.join_errors.is_empty());
            assert_eq!(spatial.join_dataset.len(), 2);
            assert!(spatial.join_dataset.iter().all(|row| row.geoid.geoid_string() == "08031000100"));
            assert_eq!(spatial.join_dataset[0].acs_value.value, "selected");
            assert_eq!(spatial.join_dataset[1].acs_value.value, "10");

            params.region = RegionFilter::Geoids(vec![Geoid::try_from("08031000100").unwrap()]);
            params.output_resolution = None;
            let explicit = acs_tiger::run_region(&params).await.unwrap();
            assert_eq!(explicit.join_dataset.len(), 2);
            assert_eq!(explicit.join_dataset[0].geometry, spatial.join_dataset[0].geometry);
            assert_eq!(explicit.join_dataset[1].acs_value.value, spatial.join_dataset[1].acs_value.value);
            let requests = tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap();
            assert!(requests[0].contains("&for=tract:*&in=state:08&in=county:031"));
            assert!(requests[1].contains("&for=tract:000100&in=state:08&in=county:031"));

            params.base_url = Some("http://127.0.0.1:1".to_string());
            params.region = region(square(6.0, 6.0, 7.0, 7.0), SpatialPredicate::Within);
            params.output_resolution = Some(GeoidType::CensusTract);
            let empty = acs_tiger::run_region(&params).await.unwrap();
            assert!(empty.join_dataset.is_empty() && empty.tiger_errors.is_empty() && empty.join_errors.is_empty());

            let dataset = LodesDataset::default().with_year(2021);
            let no_match = region(square(20.0, 20.0, 21.0, 21.0), SpatialPredicate::Within);
            let empty = lodes_tiger::run_region(&no_match, &Some(GeoidType::County), &[WacSegment::C000], &dataset, FallbackPolicy::FailAlways).await.unwrap();
            assert!(empty.join_dataset.is_empty() && empty.tiger_errors.is_empty() && empty.join_errors.is_empty());

            let territory = region(square(11.0, 11.0, 15.0, 15.0), SpatialPredicate::Within);
            let unsupported = lodes_tiger::run_region(&territory, &Some(GeoidType::County), &[WacSegment::C000], &dataset, FallbackPolicy::FailAlways).await.unwrap();
            assert!(unsupported.join_dataset.is_empty() && unsupported.join_errors.is_empty());
            assert_eq!(unsupported.tiger_errors.len(), 1);
            assert!(unsupported.tiger_errors[0].contains("72"));
        });
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    write_archive(
        temp.path(),
        "TIGER2020/COUNTY/tl_2020_us_county.zip",
        &[("08031", square(0.0, 0.0, 10.0, 10.0))],
    );
    write_archive(
        temp.path(),
        "TIGER2020/TRACT/tl_2020_08_tract.zip",
        &[
            ("08031000100", square(2.0, 2.0, 3.0, 3.0)),
            ("08031000200", square(4.0, 2.0, 5.0, 3.0)),
        ],
    );
    write_archive(
        temp.path(),
        "TIGER2021/COUNTY/tl_2021_us_county.zip",
        &[
            ("08031", square(0.0, 0.0, 10.0, 10.0)),
            ("72001", square(12.0, 12.0, 13.0, 13.0)),
        ],
    );
    run_child(
        "polygon_acs_joins_cached_geometries_and_filters_wildcard_results",
        CHILD,
        temp.path(),
        false,
    );
}

#[test]
#[ignore = "downloads LODES and TIGER data"]
fn polygon_lodes_workflow_matches_explicit_geoid_query() {
    const CHILD: &str = "BAMCENSUS_REGION_NETWORK_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let query_region = region(
                square(-104.99, 39.74, -104.98, 39.75),
                SpatialPredicate::Intersects,
            );
            let dataset = LodesDataset::default().with_year(2021);
            let wac = lodes_tiger::run_region(
                &query_region,
                &Some(GeoidType::CensusTract),
                &[WacSegment::C000],
                &dataset,
                FallbackPolicy::FailAlways,
            )
            .await
            .unwrap();
            assert!(!wac.join_dataset.is_empty());
            assert!(wac.tiger_errors.is_empty() && wac.join_errors.is_empty());
            let first = &wac.join_dataset[0];
            let explicit = lodes_tiger::run(
                &vec![first.geoid.clone()],
                &Some(GeoidType::CensusTract),
                &[WacSegment::C000],
                &dataset,
                FallbackPolicy::FailAlways,
            )
            .await
            .unwrap();
            assert_eq!(explicit.join_dataset.len(), 1);
            assert_eq!(explicit.join_dataset[0].value.value, first.value.value);
            assert_eq!(explicit.join_dataset[0].geometry, first.geometry);
        });
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    run_child(
        "polygon_lodes_workflow_matches_explicit_geoid_query",
        CHILD,
        temp.path(),
        true,
    );
}
