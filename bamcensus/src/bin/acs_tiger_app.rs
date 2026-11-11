use bamcensus::app::acs_tiger_args::AcsTigerAppCli;
use clap::Parser;

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = AcsTigerAppCli::parse();
    args.run().await
}
