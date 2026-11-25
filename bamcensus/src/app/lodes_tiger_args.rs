use super::lodes_tiger;
use super::region_args::RegionCliArgs;
use crate::model::lodes_tiger_output_row::LodesTigerOutputRow;
use bamcensus_core::model::identifier::GeoidType;
use bamcensus_lehd::model::{
    FallbackPolicy, LodesDataset, LodesEdition, LodesJobType, WacSegment, WorkplaceSegment,
};
use clap::{Args, Parser, Subcommand};
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct LodesTigerCli {
    /// declare the LODES dataset to retrieve
    #[command(subcommand)]
    pub dataset: LodesTigerDatasetCli,
}

#[derive(Subcommand, Debug)]
pub enum LodesTigerDatasetCli {
    /// Workplace-Area characteristics (WAC) LODES data downloader
    Wac(LodesTigerWacApi),
    Od,
    Rac,
}

#[derive(Args, Debug)]
pub struct LodesTigerWacApi {
    #[command(flatten)]
    pub region: RegionCliArgs,
    /// produce output rows at the given geospatial resolution. original resolution if not specified.
    #[arg(short, long)]
    pub output_resolution: Option<GeoidType>,
    /// dataset year
    #[arg(long)]
    pub year: u64,
    /// workplace area characteristic segments, see LODES documentation
    #[arg(long, default_value_t = String::from("C000"))]
    wac_segments: String,
    /// LODES definitionf, see LODES documentation, default latest
    #[arg(long, default_value = "lodes8")]
    edition: LodesEdition,
    /// LODES workforce segment defined in LODES schema documentation
    #[arg(long, default_value = "s000")]
    segment: WorkplaceSegment,
    /// WAC job type defined in LODES schema documentation
    #[arg(long, default_value = "jt00")]
    jobtype: LodesJobType,
    /// Fallback policy for handling unavailable state/year combinations:
    /// - fail_always: fail immediately (default)
    /// - try_N_years_back: try N years earlier on failure
    /// - try_N_years_forward: try N years later on failure
    /// - bidirectional_N_back_M_forward: try N years back or M years forward
    ///   Example: try_2_years_back, try_1_years_forward, bidirectional_1_back_2_forward
    #[arg(long, default_value = "fail_always", value_parser = parse_fallback_policy)]
    pub fallback: FallbackPolicy,
}

fn parse_fallback_policy(s: &str) -> Result<FallbackPolicy, String> {
    FallbackPolicy::from_str(s)
}

impl LodesTigerCli {
    pub async fn run(&self) {
        match &self.dataset {
            LodesTigerDatasetCli::Wac(wac) => run_wac(wac).await,
            LodesTigerDatasetCli::Od => todo!(),
            LodesTigerDatasetCli::Rac => todo!(),
        }
    }
}

async fn run_wac(args: &LodesTigerWacApi) {
    let wildcard = args.output_resolution;
    let region_resolution = wildcard.or(Some(GeoidType::Block));
    let region = args
        .region
        .to_region_filter(region_resolution, true)
        .unwrap();
    let dataset = LodesDataset::WAC {
        edition: args.edition,
        job_type: args.jobtype,
        segment: args.segment,
        year: args.year,
    };
    let wac_segments = args
        .wac_segments
        .split(',')
        .map(WacSegment::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let res = lodes_tiger::run_region(&region, &wildcard, &wac_segments, &dataset, args.fallback)
        .await
        .unwrap();
    println!(
        "found {} responses, {} errors",
        res.join_dataset.len(),
        res.tiger_errors.len() + res.join_errors.len(),
    );

    if !res.tiger_errors.is_empty() {
        println!("TIGER ERRORS");
        for row in res.tiger_errors.into_iter() {
            println!("{row}")
        }
    }
    if !res.join_errors.is_empty() {
        println!("DATASET JOIN ERRORS");
        for row in res.join_errors.into_iter() {
            println!("{row}")
        }
    }
    let filename = dataset.output_filename(&wildcard);
    match std::fs::remove_file(format!("{filename}.metadata.json")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("failure removing previous LODES metadata: {error}"),
    }
    let mut writer = csv::WriterBuilder::new().from_path(&filename).unwrap();
    for row in res.join_dataset {
        let out_row = LodesTigerOutputRow::from(row);
        writer.serialize(out_row).unwrap();
    }
    writer.flush().unwrap();
    write_metadata(&filename, dataset.year(), &res.selected_years).unwrap();
}

fn write_metadata(
    filename: &str,
    requested_year: u64,
    selected_years: &std::collections::BTreeMap<String, u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = serde_json::json!({
        "requested_year": requested_year,
        "selected_years": selected_years,
    });
    std::fs::write(
        format!("{filename}.metadata.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bamcensus_core::model::spatial::RegionFilter;

    #[test]
    fn metadata_sidecar_records_mixed_years_and_overwrites_previous_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let filename = temp.path().join("lodes8_wac_2020_JT00_S000_county.csv");
        let filename = filename.to_str().unwrap();
        let years =
            std::collections::BTreeMap::from([("02".to_string(), 2016), ("08".to_string(), 2020)]);
        write_metadata(filename, 2020, &years).unwrap();
        let path = format!("{filename}.metadata.json");
        let metadata: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            metadata,
            serde_json::json!({
                "requested_year": 2020,
                "selected_years": {"02": 2016, "08": 2020},
            })
        );
        write_metadata(filename, 2020, &Default::default()).unwrap();
        let metadata: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(metadata["selected_years"], serde_json::json!({}));
    }

    #[test]
    fn lodes_cli_defaults_to_all_states_and_rejects_geoid_polygon_conflict() {
        let cli = LodesTigerCli::try_parse_from(["lodes_tiger_app", "wac", "--year=2020"]).unwrap();
        let LodesTigerDatasetCli::Wac(wac) = cli.dataset else {
            panic!("expected wac subcommand");
        };
        let region = wac
            .region
            .to_region_filter(wac.output_resolution.or(Some(GeoidType::Block)), true)
            .unwrap();
        assert!(matches!(region, RegionFilter::Geoids(geoids) if geoids.len() == 51));

        assert!(LodesTigerCli::try_parse_from([
            "lodes_tiger_app",
            "wac",
            "--year=2020",
            "--geoids=08",
            "--polygon=POLYGON ((-105 39, -104 39, -104 40, -105 40, -105 39))",
        ])
        .is_err());
    }
}
