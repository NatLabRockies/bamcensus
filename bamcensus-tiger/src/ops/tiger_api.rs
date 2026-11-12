use crate::model::TigerResource;
use crate::model::TigerResourceBuilder;
use bamcensus_core::model::identifier::Geoid;
use bamcensus_core::model::identifier::GeoidType;
use futures::{stream, StreamExt};
use geo_types::Geometry;
use itertools::Itertools;
use kdam::BarExt;
use log;
use rayon::prelude::*;
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
pub async fn run(
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
            let result = read_resource(client, &tiger, |shape, record| {
                into_geoid_and_geometry(shape, record, lookup, &tiger)
            })
            .await?;

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

pub(crate) async fn read_resource<T: Send>(
    client: &Client,
    resource: &TigerResource,
    select: impl Fn(Shape, Record) -> Result<Option<T>, String> + Sync,
) -> Result<Vec<T>, String> {
    let file = get_or_download_tiger_zip(client, &resource.uri).await?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("failure reading TIGER archive {}: {e}", resource.uri))?;
    let shp_filename = get_zip_filename(&archive, ".shp")?;
    let dbf_filename = get_zip_filename(&archive, ".dbf")?;
    let shp_contents = zip_file_into_string(&mut archive, &shp_filename)?;
    let dbf_contents = zip_file_into_string(&mut archive, &dbf_filename)?;
    let mut reader = create_shapefile_reader(&shp_contents, &dbf_contents)?;
    let mut result = Vec::new();
    for chunk in &reader.iter_shapes_and_records().chunks(512) {
        let rows = chunk
            .map(|row| row.map_err(|e| format!("failure reading shapefile shape/record: {e}")))
            .collect::<Result<Vec<_>, _>>()?;
        let selected = rows
            .into_par_iter()
            .map(|(shape, record)| select(shape, record))
            .collect::<Result<Vec<_>, _>>()?;
        result.extend(selected.into_iter().flatten());
    }
    Ok(result)
}

async fn get_or_download_tiger_zip(client: &Client, uri: &str) -> Result<File, String> {
    if let Some(cache_path) = tiger_cache_path(uri) {
        return get_or_download_cached(client, uri, &cache_path).await;
    }

    let file = tempfile::tempfile()
        .map_err(|e| format!("failure creating temporary zip archive file: {e}"))?;
    download(
        client,
        uri,
        file.try_clone()
            .map_err(|e| format!("failure cloning temporary TIGER archive: {e}"))?,
    )
    .await?;
    use std::io::Seek;
    let mut file = file;
    file.rewind()
        .map_err(|e| format!("failure rewinding temporary TIGER archive: {e}"))?;
    Ok(file)
}

async fn get_or_download_cached(
    client: &Client,
    uri: &str,
    cache_path: &std::path::Path,
) -> Result<File, String> {
    if cache_path.exists() {
        log::debug!("using cached TIGER archive {}", cache_path.display());
        return File::open(cache_path).map_err(|e| {
            format!(
                "failure opening cached TIGER archive {}: {e}",
                cache_path.display()
            )
        });
    }
    let parent = cache_path
        .parent()
        .ok_or_else(|| format!("invalid TIGER cache path {}", cache_path.display()))?;
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "failure creating TIGER cache directory {}: {e}",
            parent.display()
        )
    })?;
    let temp = tempfile::Builder::new()
        .prefix(".tiger-")
        .suffix(".zip.part")
        .tempfile_in(parent)
        .map_err(|e| format!("failure creating temporary TIGER archive: {e}"))?;
    let write_file = temp
        .reopen()
        .map_err(|e| format!("failure opening temporary TIGER archive: {e}"))?;
    download(client, uri, write_file).await?;
    let archive_file = temp
        .reopen()
        .map_err(|e| format!("failure opening downloaded TIGER archive: {e}"))?;
    let archive = ZipArchive::new(archive_file)
        .map_err(|e| format!("invalid TIGER archive from {uri}: {e}"))?;
    get_zip_filename(&archive, ".shp")?;
    get_zip_filename(&archive, ".dbf")?;
    match temp.persist_noclobber(cache_path) {
        Ok(_) => {}
        Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists || cache_path.exists() => {}
        Err(e) => {
            return Err(format!(
                "failure publishing cached TIGER archive {}: {}",
                cache_path.display(),
                e.error
            ))
        }
    }
    File::open(cache_path).map_err(|e| {
        format!(
            "failure opening cached TIGER archive {}: {e}",
            cache_path.display()
        )
    })
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
pub(crate) fn get_geoid_from_record(
    record: &Record,
    geoid_type: &GeoidType,
) -> Result<Geoid, String> {
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
        .error_for_status()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;
    use zip::write::SimpleFileOptions;

    fn archive_bytes() -> Vec<u8> {
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in ["test.shp", "test.dbf"] {
            archive
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(b"fixture").unwrap();
        }
        archive.finish().unwrap().into_inner()
    }

    async fn serve_responses(
        status: &str,
        body: Vec<u8>,
        count: usize,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let uri = format!("http://{}/archive.zip", listener.local_addr().unwrap());
        let status = status.to_string();
        let handle = tokio::spawn(async move {
            let mut sockets = Vec::new();
            for _ in 0..count {
                sockets.push(listener.accept().await.unwrap().0);
            }
            for mut socket in sockets {
                let mut reader = BufReader::new(&mut socket);
                let mut line = String::new();
                loop {
                    line.clear();
                    assert!(reader.read_line(&mut line).await.unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                }
                let headers = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                socket.write_all(headers.as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            }
        });
        (uri, handle)
    }

    #[tokio::test]
    async fn failed_responses_do_not_poison_cache() {
        let dir = tempfile::tempdir().unwrap();
        for (status, body) in [
            ("404 Not Found", b"missing".to_vec()),
            ("200 OK", b"not a zip".to_vec()),
        ] {
            let (uri, server) = serve_responses(status, body, 1).await;
            let cache = dir.path().join("archive.zip");
            let result = get_or_download_cached(&Client::new(), &uri, &cache).await;
            assert!(result.is_err());
            assert!(!cache.exists());
            server.await.unwrap();
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
        }
    }

    #[tokio::test]
    async fn concurrent_downloads_publish_a_complete_archive() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("archive.zip");
        let bytes = archive_bytes();
        let (uri, server) = serve_responses("200 OK", bytes.clone(), 2).await;
        let client = Client::new();
        let (first, second) = tokio::join!(
            get_or_download_cached(&client, &uri, &cache),
            get_or_download_cached(&client, &uri, &cache)
        );
        first.unwrap();
        second.unwrap();
        server.await.unwrap();
        assert_eq!(fs::read(&cache).unwrap(), bytes);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
