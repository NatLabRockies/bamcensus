use bamcensus_core::model::identifier::{fips, Geoid, GeoidType, HasGeoidString};
use bamcensus_core::model::spatial::SpatialPredicate;
use bamcensus_tiger::model::TigerResourceBuilder;
use bamcensus_tiger::ops::{
    geometry_io::parse_geometry_input, spatial_resolver::resolve_polygon, tiger_api,
};
use geo_types::{polygon, Geometry, Polygon};
use shapefile::dbase::{FieldValue, Record, TableWriterBuilder};
use std::{fs, io::Write, path::Path, process::Command};
use zip::{write::SimpleFileOptions, ZipWriter};

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon<f64> {
    polygon![(x: x0, y: y0), (x: x1, y: y0), (x: x1, y: y1), (x: x0, y: y1), (x: x0, y: y0)]
}

fn write_archive(root: &Path, relative_path: &str, records: &[(&str, Polygon<f64>)]) {
    let shp = root.join("fixture.shp");
    let fields = TableWriterBuilder::new().add_character_field("GEOID".try_into().unwrap(), 16);
    {
        let mut writer = shapefile::Writer::from_path(&shp, fields).unwrap();
        for (geoid, polygon) in records {
            let mut record = Record::default();
            record.insert(
                "GEOID".to_string(),
                FieldValue::Character(Some(geoid.to_string())),
            );
            let shape = shapefile::Polygon::from(polygon.clone());
            writer.write_shape_and_record(&shape, &record).unwrap();
        }
    }
    let archive_path = root.join("tiger").join(relative_path);
    fs::create_dir_all(archive_path.parent().unwrap()).unwrap();
    let mut zip = ZipWriter::new(fs::File::create(archive_path).unwrap());
    for extension in ["shp", "dbf"] {
        zip.start_file(format!("fixture.{extension}"), SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&fs::read(shp.with_extension(extension)).unwrap())
            .unwrap();
    }
    zip.finish().unwrap();
}

fn build_fixtures(root: &Path) {
    let donut = Polygon::new(
        square(30.0, 0.0, 40.0, 10.0).exterior().clone(),
        vec![square(34.0, 4.0, 36.0, 6.0).exterior().clone()],
    );
    write_archive(
        root,
        "TIGER2020/COUNTY/tl_2020_us_county.zip",
        &[
            ("08031", square(0.0, 0.0, 10.0, 10.0)),
            ("08059", square(10.0, 0.0, 20.0, 10.0)),
            ("06001", donut.clone()),
        ],
    );
    write_archive(
        root,
        "TIGER2020/TRACT/tl_2020_06_tract.zip",
        &[("06001000100", donut)],
    );
    write_archive(
        root,
        "TIGER2020/TRACT/tl_2020_08_tract.zip",
        &[
            ("08031000100", square(2.0, 2.0, 3.0, 3.0)),
            ("08031000200", square(4.0, 2.0, 5.0, 3.0)),
            ("08059000100", square(12.0, 2.0, 13.0, 3.0)),
        ],
    );
    for (path, geoid) in [
        ("TIGER2020/STATE/tl_2020_us_state.zip", "08"),
        ("TIGER2020/PLACE/tl_2020_08_place.zip", "0812345"),
        ("TIGER2020/COUSUB/tl_2020_08_cousub.zip", "0803112345"),
        ("TIGER2020/BG/tl_2020_08_bg.zip", "080310001001"),
        (
            "TIGER2020/TABBLOCK20/tl_2020_08_tabblock20.zip",
            "080310001000001",
        ),
    ] {
        write_archive(root, path, &[(geoid, square(2.0, 2.0, 3.0, 3.0))]);
    }

    let mut states = Geoid::all_states();
    states.extend([60, 66, 69, 72, 78].map(|s| Geoid::State(fips::State(s))));
    for state in states {
        let code = state.geoid_string();
        let polygon = if code == "08" {
            square(0.0, 0.0, 10.0, 10.0)
        } else {
            square(30.0, 30.0, 31.0, 31.0)
        };
        write_archive(
            root,
            &format!("TIGER2010/STATE/2010/tl_2010_{code}_state10.zip"),
            &[(&code, polygon)],
        );
    }
    write_archive(
        root,
        "TIGER2010/COUNTY/2010/tl_2010_08_county10.zip",
        &[("08031", square(0.0, 0.0, 10.0, 10.0))],
    );
    write_archive(
        root,
        "TIGER2010/TRACT/2010/tl_2010_08031_tract10.zip",
        &[("08031000100", square(2.0, 2.0, 3.0, 3.0))],
    );
    assert!(parse_geometry_input(root.join("fixture.shp").to_str().unwrap()).is_ok());
}

#[test]
fn resolves_cached_archives_offline_for_both_vintages() {
    const CHILD: &str = "BAMCENSUS_TEST_SPATIAL_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all("http://127.0.0.1:1").unwrap())
            .build()
            .unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let query = Geometry::Polygon(square(1.0, 1.0, 4.0, 4.0));
        runtime.block_on(async {
            for year in [2010, 2020] {
                let builder = TigerResourceBuilder::new(year).unwrap();
                let within = resolve_polygon(
                    &client,
                    &builder,
                    &query,
                    GeoidType::CensusTract,
                    SpatialPredicate::Within,
                )
                .await
                .unwrap();
                assert!(within
                    .parent_geoids
                    .contains(&Geoid::try_from("08031").unwrap()));
                assert_eq!(
                    within
                        .matched_geometries
                        .iter()
                        .map(|(g, _)| g.geoid_string())
                        .collect::<Vec<_>>(),
                    vec!["08031000100"]
                );
                let centroid = resolve_polygon(
                    &client,
                    &builder,
                    &query,
                    GeoidType::CensusTract,
                    SpatialPredicate::CentroidWithin,
                )
                .await
                .unwrap();
                assert_eq!(centroid.matched_geometries.len(), 1);
                let intersects = resolve_polygon(
                    &client,
                    &builder,
                    &query,
                    GeoidType::CensusTract,
                    SpatialPredicate::Intersects,
                )
                .await
                .unwrap();
                assert_eq!(
                    intersects.matched_geometries.len(),
                    if year == 2020 { 2 } else { 1 }
                );
                let geoid = Geoid::try_from("08031000100").unwrap();
                let existing = tiger_api::run(&client, &builder, &[&geoid]).await.unwrap();
                assert_eq!(existing.into_iter().next().unwrap().unwrap().len(), 1);
            }

            let builder = TigerResourceBuilder::new(2020).unwrap();
            for (target, expected) in [
                (GeoidType::State, "08"),
                (GeoidType::County, "08031"),
                (GeoidType::Place, "0812345"),
                (GeoidType::CountySubdivision, "0803112345"),
                (GeoidType::BlockGroup, "080310001001"),
                (GeoidType::Block, "080310001000001"),
            ] {
                let result = resolve_polygon(
                    &client,
                    &builder,
                    &query,
                    target,
                    SpatialPredicate::Intersects,
                )
                .await
                .unwrap();
                assert_eq!(
                    result
                        .matched_geometries
                        .iter()
                        .map(|(g, _)| g.geoid_string())
                        .collect::<Vec<_>>(),
                    vec![expected]
                );
            }
            let no_match = Geometry::Polygon(square(-10.0, -10.0, -9.0, -9.0));
            let empty = resolve_polygon(
                &client,
                &builder,
                &no_match,
                GeoidType::CensusTract,
                SpatialPredicate::Intersects,
            )
            .await
            .unwrap();
            assert!(empty.matched_geometries.is_empty());
            assert!(empty.parent_geoids.is_empty());

            let query_in_hole = Geometry::Polygon(square(34.5, 4.5, 35.5, 5.5));
            let result = resolve_polygon(
                &client,
                &TigerResourceBuilder::new(2020).unwrap(),
                &query_in_hole,
                GeoidType::CensusTract,
                SpatialPredicate::CentroidWithin,
            )
            .await
            .unwrap();
            assert!(result
                .parent_geoids
                .contains(&Geoid::try_from("06001").unwrap()));
            assert_eq!(
                result
                    .matched_geometries
                    .iter()
                    .map(|(g, _)| g.geoid_string())
                    .collect::<Vec<_>>(),
                vec!["06001000100"]
            );
        });
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    build_fixtures(temp.path());
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "resolves_cached_archives_offline_for_both_vintages",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("BAMCENSUS_CACHE_DIR", temp.path())
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
#[ignore = "downloads 2020 TIGER state, county, and Colorado tract archives"]
fn resolves_a_real_state_archive() {
    const CHILD: &str = "BAMCENSUS_TEST_SPATIAL_NETWORK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let client = reqwest::Client::new();
        let query = Geometry::Polygon(square(-104.99, 39.74, -104.98, 39.75));
        let builder = TigerResourceBuilder::new(2020).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let result = resolve_polygon(
                &client,
                &builder,
                &query,
                GeoidType::State,
                SpatialPredicate::Intersects,
            )
            .await
            .unwrap();
            assert_eq!(
                result
                    .matched_geometries
                    .iter()
                    .map(|(g, _)| g.geoid_string())
                    .collect::<Vec<_>>(),
                vec!["08"]
            );
            let offline_client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all("http://127.0.0.1:1").unwrap())
                .build()
                .unwrap();
            let cached = resolve_polygon(
                &offline_client,
                &builder,
                &query,
                GeoidType::State,
                SpatialPredicate::Intersects,
            )
            .await
            .unwrap();
            assert_eq!(cached.parent_geoids, result.parent_geoids);

            let tracts = resolve_polygon(
                &client,
                &builder,
                &query,
                GeoidType::CensusTract,
                SpatialPredicate::Intersects,
            )
            .await
            .unwrap();
            assert!(!tracts.matched_geometries.is_empty());
            assert!(tracts
                .parent_geoids
                .contains(&Geoid::try_from("08031").unwrap()));
            let matched_geoid = &tracts.matched_geometries[0].0;
            let direct = tiger_api::run(&offline_client, &builder, &[matched_geoid])
                .await
                .unwrap();
            assert_eq!(
                direct.into_iter().next().unwrap().unwrap()[0].0,
                *matched_geoid
            );
            let cached_tracts = resolve_polygon(
                &offline_client,
                &builder,
                &query,
                GeoidType::CensusTract,
                SpatialPredicate::Intersects,
            )
            .await
            .unwrap();
            assert_eq!(cached_tracts.matched_geometries, tracts.matched_geometries);
        });
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "resolves_a_real_state_archive",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("BAMCENSUS_CACHE_DIR", temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
