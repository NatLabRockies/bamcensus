use bamcensus_core::model::identifier::{Geoid, GeoidType};
use bamcensus_core::ops::agg::NumericAggregation;
use bamcensus_lehd::api::lodes_api;
use bamcensus_lehd::model::{
    LodesDataset, LodesEdition, LodesJobType, WacSegment, WorkplaceSegment,
};
use clap::Parser;

// todo: top level here should be a LEHD command
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct LodesCliArgs {
    /// LODES year in [2002, 2016]
    #[arg(short, long)]
    year: u64,
    /// states to download by FIPS code. omit to download all states.
    #[arg(short, long)]
    geoids: Option<String>,
    /// LODES are created in editions, see website for details. LODES8 by default if not provided.
    #[arg(long)]
    edition: Option<LodesEdition>,
    /// LODES workforce segment defined in LODES schema documentation
    #[arg(long)]
    segment: Option<WorkplaceSegment>,
    /// WAC job type defined in LODES schema documentation
    #[arg(long)]
    jobtype: Option<LodesJobType>,
    /// level to aggregate result value
    #[arg(long)]
    agg_geoid_type: Option<GeoidType>,
    /// function to aggregate result value
    #[arg(long)]
    agg_fn: Option<NumericAggregation>,
    // todo: use clap.Parser's subcommand structures to flip between WAC, OD, and RAC data since they
    // are structurally different
}

impl LodesCliArgs {
    pub fn get_state_geoids(&self) -> Result<Vec<Geoid>, String> {
        match &self.geoids {
            Some(s) => s
                .split(',')
                .map(Geoid::try_from)
                .collect::<Result<Vec<_>, _>>(),
            None => Ok(Geoid::all_states()),
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = LodesCliArgs::parse();

    let edition = args.edition.unwrap_or_default();
    let segment = args.segment.unwrap_or_default();
    let job_type = args.jobtype.unwrap_or_default();
    let year = args.year;
    let dataset = LodesDataset::WAC {
        edition,
        job_type,
        segment,
        year,
    };
    let wac_segments = vec![WacSegment::C000];
    let state_codes = args.get_state_geoids().unwrap();
    let agg_fn = args.agg_fn.unwrap_or_default();
    let output_geoid_type = args.agg_geoid_type.unwrap_or(GeoidType::Block);
    let queries = state_codes
        .iter()
        .map(|s| dataset.create_query(s))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("executing LODES download");
    let client = reqwest::Client::new();
    let agg_rows = lodes_api::run_wac(
        &client,
        &queries,
        &wac_segments,
        Some((output_geoid_type, agg_fn)),
    )
    .await
    .unwrap();

    let n_res = agg_rows.len();
    println!("{n_res} agg rows");
    println!("queries:");
    for (geoid, values) in agg_rows.iter() {
        println!("{geoid}");
        for value in values.iter() {
            println!("  {value}");
        }
    }
}
