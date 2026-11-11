//! Command line tool for running BAMCENSUS Apps.
use bamcensus::app::acs_tiger_args::AcsTigerAppCli;
use bamcensus::app::lodes_tiger_args::LodesTigerCli;
use clap::Parser;
use clap::Subcommand;

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
    #[command(alias = "acs")]
    AcsApp(AcsTigerAppCli),
    /// Longitudinal Employer-Household Dynamics (LEHD) Download Tools
    #[command(subcommand, alias = "lehd")]
    LehdApp(LehdAppCli),
}

#[derive(Subcommand)]
pub enum LehdAppCli {
    Lodes(LodesTigerCli),
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = BamCensusCli::parse();
    match args.command {
        BamCensusApp::AcsApp(acs_args) => acs_args.run().await,
        BamCensusApp::LehdApp(LehdAppCli::Lodes(lodes_args)) => lodes_args.run().await,
    }
}
