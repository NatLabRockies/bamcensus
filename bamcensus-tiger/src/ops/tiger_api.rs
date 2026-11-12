use crate::model::TigerResource;
use crate::model::TigerResourceBuilder;
use bamcensus_core::model::identifier::Geoid;
use bamcensus_core::model::identifier::GeoidType;
use futures::{stream, StreamExt};
use geo_types::Geometry;
use itertools::Itertools;
use kdam::BarExt;
use log;
use reqwest::Client;
use shapefile::dbase::Record;
use shapefile::{dbase, Shape, ShapeReader};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

const MAX_CONCURRENT_DOWNLOADS: usize = 8;

/// runs as many downloads of TIGER/Lines files as needed to cover
/// the target geoids. return only rows matching the requested geoids.
///
/// this requires downloading .zip archives from the TIGER/Lines HTTP
/// site. the archives are Shapefile archives, and there isn't great
/// tooling to stream these data sources, so here we chose to download
/// the archives, unpack, and then load from the extracted file paths.
pub async fn run<'a>(
    client: &Client,
    builder: &TigerResourceBuilder,
    geoids: &[&Geoid],
) -> Result<Vec<Result<Vec<(Geoid, Geometry)>, String>>, String> {
    let uris = builder.create_resources(geoids)?;
    let lookup = geoids.iter().collect::<HashSet<_>>();

    let pb_builder = kdam::BarBuilder::default()
        .total(uris.len())
        .desc("TIGER/Lines downloads");
    let pb = Arc::new(Mutex::new(pb_builder.build()?));

    let run_results = uris.into_iter().map(|tiger| {
        log::debug!("downloading {}", tiger.uri);
        let client = &client;
        let lookup = &lookup;
        let pb = pb.clone();
        async move {
            let read_path = get_or_download_tiger_zip(client, &tiger.uri).await?;

            // unpack archive
            let read_file = File::open(&read_path)
                .map_err(|e| format!("failure opening temporary zip archive file location: {e}"))?;
            let mut z = ZipArchive::new(read_file)
                .map_err(|e| format!("failure reading temporary zip archive: {e}"))?;
            let shp_filename = get_zip_filename(&z, ".shp")?;
            let dbf_filename = get_zip_filename(&z, ".dbf")?;
            let shp_contents = zip_file_into_string(&mut z, &shp_filename)?;
            let dbf_contents = zip_file_into_string(&mut z, &dbf_filename)?;

            // read shapes and records
            let mut reader = create_shapefile_reader(&shp_contents, &dbf_contents)?;
            let read_result = reader
                .iter_shapes_and_records()
                .map(|row| {
                    let (shape, record) =
                        row.map_err(|e| format!("failure reading shapefile shape/record: {e}"))?;
                    into_geoid_and_geometry(shape, record, lookup, &tiger)
                })
                .collect::<Result<Vec<_>, String>>()?;
            let result = read_result.into_iter().flatten().collect_vec();

            // update progress bar
            let mut pb_update = pb
                .lock()
                .map_err(|e| format!("failure acquiring progress bar mutex lock: {e}"))?;
            pb_update
                .update(1)
                .map_err(|e| format!("failure on pb update: {e}"))?;
            pb_update.set_description(tiger.uri.split('/').next_back().unwrap_or_default());

            Ok(result)
        }
    });
    let result = stream::iter(run_results)
        .buffer_unordered(MAX_CONCURRENT_DOWNLOADS)
        .collect::<Vec<_>>()
        .await;
    eprintln!(); // terminate progress bar
    Ok(result)
}

async fn get_or_download_tiger_zip(client: &Client, uri: &str) -> Result<PathBuf, String> {
    if let Some(cache_path) = tiger_cache_path(uri) {
        if cache_path.exists() {
            log::debug!("using cached TIGER archive {}", cache_path.display());
            return Ok(cache_path);
        }
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
            if let Ok(named_tmp) = tempfile::Builder::new()
                .prefix(".tiger-")
                .suffix(".zip.part")
                .tempfile_in(parent)
            {
                let (file, tmp_path) = named_tmp.into_parts();
                download(client, uri, file).await?;
                match tmp_path.persist(&cache_path) {
                    Ok(_) => return Ok(cache_path),
                    Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                        return Ok(cache_path);
                    }
                    Err(e) => {
                        if cache_path.exists() {
                            return Ok(cache_path);
                        }
                        return Err(format!(
                            "failure moving cached zip file into place: {}",
                            e.error
                        ));
                    }
                }
            }
        }
    }

    let named_tmp = tempfile::NamedTempFile::new()
        .map_err(|e| format!("failure creating temporary zip archive filepath: {e}"))?;
    let (write_file, tmp_path) = named_tmp.into_parts();
    download(client, uri, write_file).await?;
    let read_path = tmp_path
        .keep()
        .map_err(|e| format!("failure persisting temporary zip archive file: {e}"))?;
    Ok(read_path)
}

fn tiger_cache_path(uri: &str) -> Option<PathBuf> {
    let base_dir = std::env::var_os("BAMCENSUS_CACHE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(|home| PathBuf::from(home).join(".cache").join("bamcensus"))
        })?;
    let relative = uri
        .strip_prefix(TigerResourceBuilder::TIGER_BASE_URL)
        .unwrap_or(uri)
        .trim_start_matches('/');
    Some(base_dir.join("tiger").join(relative))
}

fn into_geoid_and_geometry(
    shape: Shape,
    record: Record,
    lookup: &HashSet<&&Geoid>,
    tiger_uri: &TigerResource,
) -> Result<Option<(Geoid, Geometry)>, String> {
    let geoid = get_geoid_from_record(&record, &tiger_uri.geoid_type)?;
    if lookup.contains(&&geoid) {
        let geometry: Geometry<f64> = shape
            .try_into()
            .map_err(|e| format!("could not convert shape into geometry. {e}"))?;
        Ok(Some((geoid, geometry)))
    } else {
        Ok(None)
    }
}

const GEOID_COLUMN_NAMES: [&str; 3] = ["GEOID", "GEOID20", "GEOID10"];

/// attempts all three conventions for GEOID column names. order is:
/// 1. "GEOID"   - most general
/// 2. "GEOID20" - latest
/// 3. "GEOID10" - when general or latest is not present
fn get_geoid_from_record(record: &Record, geoid_type: &GeoidType) -> Result<Geoid, String> {
    let field_name = GEOID_COLUMN_NAMES
        .iter()
        .find(|col| record.get(col).is_some())
        .ok_or_else(|| {
            format!(
                "could not find any of {} in shapefile",
                GEOID_COLUMN_NAMES.iter().join(","),
            )
        })?;
    let field_value = record.get(field_name).ok_or_else(|| {
        format!(
            "could not find any of {} in shapefile",
            GEOID_COLUMN_NAMES.iter().join(","),
        )
    })?;
    let geoid = match field_value {
        dbase::FieldValue::Character(s) => match s {
            Some(geoid_string) => geoid_type.geoid_from_str(geoid_string),
            None => Err(format!(
                "value at Geoid field '{field_name}' is empty, should be a GEOID string"
            )),
        },
        _ => Err(format!(
            "value at column '{field_name}' is not valid GEOID, found '{field_value}'"
        )),
    }?;
    Ok(geoid)
}

async fn download(client: &Client, uri: &str, write_file: File) -> Result<(), String> {
    let mut async_file = tokio::fs::File::from(write_file);

    let mut response = client
        .get(uri)
        .send()
        .await
        .map_err(|e| format!("failure retrieving TIGER zip archive: {e}"))?
        .bytes_stream();

    while let Some(buf) = response.next().await {
        let item = buf.map_err(|e| format!("failed to buffer response: {e}"))?;
        tokio::io::copy(&mut item.as_ref(), &mut async_file)
            .await
            .map_err(|e| format!("failed to write response buffer: {e}"))?;
    }

    async_file
        .flush()
        .await
        .map_err(|e| format!("error closing async write connection to temp zip file: {e}"))?;
    Ok(())
}

fn get_zip_filename(archive: &ZipArchive<File>, suffix: &str) -> Result<String, String> {
    let shp_filename = archive
        .file_names()
        .find(|s| s.ends_with(suffix))
        .ok_or_else(|| format!("no files in archive have '{suffix}' suffix"))?;
    Ok(String::from(shp_filename))
}

fn zip_file_into_string(archive: &mut ZipArchive<File>, filename: &str) -> Result<Vec<u8>, String> {
    let mut contents = Vec::new();
    let mut zipfile = archive.by_name(filename).map_err(|e| {
        format!("expected file {filename} cannot be retrieved by name from zip archive: {e}")
    })?;
    zipfile
        .read_to_end(&mut contents)
        .map_err(|e| format!("failure reading {filename} from zip archive: {e}"))?;
    // let string =
    //     String::from_utf8(contents).map_err(|e| format!("failure parsing zip as utf-8: {}", e))?;
    Ok(contents)
}

type TigerShapefileReader<'a> =
    Result<shapefile::Reader<Cursor<&'a Vec<u8>>, Cursor<&'a Vec<u8>>>, String>;
fn create_shapefile_reader<'a>(
    shp_contents: &'a Vec<u8>,
    dbf_contents: &'a Vec<u8>,
) -> TigerShapefileReader<'a> {
    let shp_cursor = Cursor::new(shp_contents);
    let dbf_cursor = Cursor::new(dbf_contents);
    let shape_reader =
        ShapeReader::new(shp_cursor).map_err(|e| format!("failure building shape reader: {e}"))?;
    let database_reader =
        dbase::Reader::new(dbf_cursor).map_err(|e| format!("failure building dbf reader: {e}"))?;
    let reader: shapefile::Reader<Cursor<&Vec<u8>>, Cursor<&Vec<u8>>> =
        shapefile::Reader::new(shape_reader, database_reader);
    Ok(reader)
}
