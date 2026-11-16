use super::acs_tiger::{self, AcsRegionQueryParams};
use super::region_args::RegionCliArgs;
use crate::model::acs_tiger_output_row::AcsTigerOutputRow;
use bamcensus_acs::model::AcsType;
use bamcensus_core::model::identifier::GeoidType;
use clap::Parser;
use itertools::Itertools;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AcsTigerAppCli {
    #[command(flatten)]
    pub region: RegionCliArgs,
    /// produce output rows at the given geospatial resolution. original resolution if not specified.
    #[arg(short, long, alias = "aggregation")]
    pub output_resolution: Option<GeoidType>,
    /// year of ACS data / tiger lines data to retrieve
    #[arg(long)]
    pub year: u64,
    /// ACS data column to retrieve, see ACS documentation for columns by year/type
    #[arg(long)]
    pub acs_query: String,
    /// one or five year estimates, see ACS documentation for more information
    #[arg(short, long)]
    pub acs_type: AcsType,
    /// token for ACS API (can also be set via CENSUS_API_TOKEN)
    #[arg(short = 't', long, env = "CENSUS_API_TOKEN")]
    pub acs_token: String,
    /// path and file to write result. if not provided, will use a concatenation of the CLI arguments
    #[arg(long)]
    pub output_file: Option<String>,
}

impl AcsTigerAppCli {
    pub async fn run(&self) {
        let acs_get_query = self.acs_query.split(',').map(String::from).collect_vec();
        let region = self
            .region
            .to_region_filter(self.output_resolution, false)
            .unwrap();
        let params = AcsRegionQueryParams {
            base_url: None,
            year: self.year,
            acs_type: self.acs_type,
            acs_get_query,
            region,
            output_resolution: self.output_resolution,
            acs_token: Some(self.acs_token.clone()),
        };

        let res_msg = match self.output_resolution {
            Some(res) => res.to_string(),
            None => String::new(),
        };
        let filename = match &self.output_file {
            None => {
                if res_msg.is_empty() {
                    format!("{}-{}.csv", self.year, self.acs_type)
                } else {
                    format!("{}-{}-{}.csv", self.year, self.acs_type, res_msg)
                }
            }
            Some(f) => f.clone(),
        };
        let res = acs_tiger::run_region(&params).await.unwrap();
        let total_errors = res.tiger_errors.len() + res.join_errors.len();
        println!(
            "found {} responses, {} errors",
            res.join_dataset.len(),
            total_errors
        );
        if !res.tiger_errors.is_empty() {
            println!("TIGER ERRORS");
            for row in res.tiger_errors.into_iter() {
                println!("{row}")
            }
        }

        if !res.join_errors.is_empty() {
            println!("JOIN ERRORS");
            for row in res.join_errors.into_iter() {
                println!("{row}")
            }
        }

        let mut writer = csv::WriterBuilder::new().from_path(filename).unwrap();
        for row in res.join_dataset {
            let out_row = AcsTigerOutputRow::from(row);
            writer.serialize(out_row).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_rejects_conflicting_geoids_and_polygon_and_missing_polygon_resolution() {
        assert!(AcsTigerAppCli::try_parse_from([
            "acs_tiger_app",
            "--geoids=08031",
            "--polygon=POLYGON ((-105 39, -104 39, -104 40, -105 40, -105 39))",
            "--year=2020",
            "--acs-query=NAME",
            "--acs-type=five-year",
            "--acs-token=test-token",
        ])
        .is_err());

        let parsed = AcsTigerAppCli::try_parse_from([
            "acs_tiger_app",
            "--polygon=POLYGON ((-105 39, -104 39, -104 40, -105 40, -105 39))",
            "--year=2020",
            "--acs-query=NAME",
            "--acs-type=five-year",
            "--acs-token=test-token",
        ])
        .unwrap();
        assert_eq!(parsed.acs_token, "test-token");
        assert!(parsed
            .region
            .to_region_filter(parsed.output_resolution, false)
            .is_err());
    }
}
