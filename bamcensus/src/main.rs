//! Command line tool for running BAMCENSUS Apps.
use bamcensus::app::acs_tiger;
use bamcensus::app::lodes_tiger_args::LodesTigerCli;
use bamcensus::model::acs_tiger_output_row::AcsTigerOutputRow;
use bamcensus_acs::model::{AcsApiQueryParams, AcsGeoidQuery, AcsType};
use bamcensus_core::model::identifier::Geoid;
use bamcensus_core::model::identifier::GeoidType;
use clap::Parser;
use clap::Subcommand;
use itertools::Itertools;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct BamCensusCli {
    #[command(subcommand)]
    pub command: BamCensusApp,
}

#[derive(Subcommand)]
pub enum BamCensusApp {
    /// American Community Survey (ACS) Download Tools
    AcsApp(AcsAppCli),
    /// Longitudinal Employer-Household Dynamics (LEHD) Download Tools
    #[command(subcommand)]
    LehdApp(LehdAppCli),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AcsAppCli {
    /// geoid describing the download region
    #[arg(short, long)]
    pub geoid: String,
    /// level to aggregate results. no aggregation if not provided.
    #[arg(short, long)]
    pub aggregation: Option<GeoidType>,
    /// dataset year. must match known ACS releases.
    #[arg(long)]
    pub year: u64,
    /// comma-delimited list of ACS fields to retrieve
    #[arg(long)]
    pub acs_query: String,
    /// one or five-year ACS dataset
    #[arg(short, long)]
    pub acs_type: AcsType,
    /// optional API token, may be required depending on server limits.
    #[arg(short, long)]
    pub acs_token: Option<String>,
}

#[derive(Subcommand)]
pub enum LehdAppCli {
    Lodes(LodesTigerCli),
}

#[tokio::main]
async fn main() {
    let args = BamCensusCli::parse();
    match args.command {
        BamCensusApp::AcsApp(acs_args) => acs(&acs_args).await,
        BamCensusApp::LehdApp(LehdAppCli::Lodes(lodes_args)) => lodes_args.run().await,
    }
}

async fn acs(args: &AcsAppCli) {
    let acs_get_query = args.acs_query.split(',').map(String::from).collect_vec();
    let geoid = Geoid::try_from(args.geoid.as_str()).unwrap();
    let query: AcsGeoidQuery = AcsGeoidQuery::new(Some(geoid), args.aggregation).unwrap();
    let query_params = AcsApiQueryParams::new(
        None,
        args.year,
        args.acs_type,
        acs_get_query,
        query,
        args.acs_token.clone(),
    );

    let filename = &query_params.output_filename();
    let res = acs_tiger::run(&query_params).await.unwrap();
    println!(
        "found {} responses, {}/{} errors",
        res.join_dataset.len(),
        res.tiger_errors.len(),
        res.join_errors.len(),
    );
    println!("TIGER ERRORS");
    for row in res.tiger_errors.into_iter() {
        println!("{row}")
    }
    println!("JOIN ERRORS");
    for row in res.join_errors.into_iter() {
        println!("{row}")
    }

    let mut writer = csv::WriterBuilder::new().from_path(filename).unwrap();
    for row in res.join_dataset {
        let out_row = AcsTigerOutputRow::from(row);
        writer.serialize(out_row).unwrap();
    }
}
