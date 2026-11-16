use super::region_args::RegionCliArgs;
use bamcensus_core::model::identifier::{GeoidType, HasGeoidString};
use bamcensus_tiger::model::TigerResourceBuilder;
use bamcensus_tiger::ops::geometry_io::parse_geometry_input;
use bamcensus_tiger::ops::spatial_resolver;
use clap::Parser;
use reqwest::Client;
use serde::Serialize;
use wkt::ToWkt;

#[derive(Parser, Debug)]
#[command(author, version, about = "Resolve a polygon to matching TIGER/Line GEOIDs", long_about = None)]
pub struct TigerAppCli {
    /// polygonal query region (inline WKT/GeoJSON, or path to .wkt, .geojson, .json, or .shp file)
    #[arg(long)]
    pub polygon: String,
    /// target geospatial resolution (state, county, county-subdivision, place, census-tract, block-group, block)
    #[arg(short, long, alias = "aggregation")]
    pub output_resolution: GeoidType,
    /// TIGER/Line vintage year
    #[arg(long)]
    pub year: u64,
    /// spatial predicate for polygon queries: intersects, centroid-within, within, or area-overlap[:fraction]
    #[arg(long, default_value = "intersects")]
    pub spatial_predicate: String,
    /// minimum overlap fraction in (0, 1] when --spatial-predicate is area-overlap
    #[arg(long)]
    pub min_overlap: Option<f64>,
    /// include WKT geometry column in output
    #[arg(long, default_value_t = false)]
    pub include_geometry: bool,
    /// optional CSV file path to write results; prints to stdout if omitted
    #[arg(long)]
    pub output_file: Option<String>,
}

#[derive(Serialize)]
struct TigerGeoidRow {
    geoid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    geometry: Option<String>,
}

impl TigerAppCli {
    pub async fn run(&self) {
        let region_args = RegionCliArgs {
            geoids: None,
            polygon: Some(self.polygon.clone()),
            spatial_predicate: self.spatial_predicate.clone(),
            min_overlap: self.min_overlap,
        };
        let predicate = region_args.parse_predicate().unwrap();
        let geometry = parse_geometry_input(&self.polygon).unwrap();
        let builder = TigerResourceBuilder::new(self.year).unwrap();
        let client = Client::new();
        let resolved = spatial_resolver::resolve_polygon(
            &client,
            &builder,
            &geometry,
            self.output_resolution,
            predicate,
        )
        .await
        .unwrap();

        let rows: Vec<TigerGeoidRow> = resolved
            .matched_geometries
            .into_iter()
            .map(|(geoid, geom)| TigerGeoidRow {
                geoid: geoid.geoid_string(),
                geometry: self.include_geometry.then(|| geom.to_wkt().to_string()),
            })
            .collect();

        match &self.output_file {
            Some(path) => {
                let mut writer = csv::WriterBuilder::new().from_path(path).unwrap();
                for row in &rows {
                    writer.serialize(row).unwrap();
                }
                println!("wrote {} matching GEOIDs to {}", rows.len(), path);
            }
            None => {
                let mut writer = csv::WriterBuilder::new().from_writer(std::io::stdout());
                for row in &rows {
                    writer.serialize(row).unwrap();
                }
                writer.flush().unwrap();
            }
        }
    }
}
