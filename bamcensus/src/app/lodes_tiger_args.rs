use super::lodes_tiger;
use crate::model::lodes_tiger_output_row::LodesTigerOutputRow;
use bamcensus_core::model::identifier::GeoidType;
use bamcensus_core::model::identifier::{Geoid, StateCode};
use bamcensus_lehd::model::{
    LodesDataset, LodesEdition, LodesJobType, FallbackPolicy, WacSegment, WorkplaceSegment,
};
use clap::{Args, Parser, Subcommand};
use itertools::Itertools;
use std::str::FromStr;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct LodesTigerCli {
    /// declare the LODES dataset to retrieve
    #[command(subcommand)]
    dataset: LodesTigerDatasetCli,
}

#[derive(Subcommand)]
pub enum LodesTigerDatasetCli {
    /// Workplace-Area characteristics (WAC) LODES data downloader
    Wac(LodesTigerWacApi),
    Od,
    Rac,
}

#[derive(Args)]
pub struct LodesTigerWacApi {
    /// comma-delimited list of geoids representing the geographic area for download
    #[arg(short, long)]
    pub geoids: Option<String>,
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
    /// Example: try_2_years_back, try_1_years_forward, bidirectional_1_back_2_forward
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
    let geoids = match &args.geoids {
        Some(s) => s
            .split(',')
            .map(|g| Geoid::try_from(g).unwrap())
            .collect_vec(),
        None => StateCode::ALL
            .iter()
            .map(|sc| {
                let fips = sc.to_fips_string();
                Geoid::try_from(fips.as_str()).unwrap()
            })
            .collect_vec(),
    };
    let dataset = LodesDataset::WAC {
        edition: args.edition,
        job_type: args.jobtype,
        segment: args.segment,
        year: args.year,
    };
    let wildcard = args.output_resolution;
    let wac_segments = args
        .wac_segments
        .split(',')
        .map(WacSegment::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let res = lodes_tiger::run(&geoids, &wildcard, &wac_segments, &dataset, args.fallback)
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
    let mut writer = csv::WriterBuilder::new()
        .from_path(dataset.output_filename(&wildcard))
        .unwrap();
    for row in res.join_dataset {
        let out_row = LodesTigerOutputRow::from(row);
        writer.serialize(out_row).unwrap();
    }
}
