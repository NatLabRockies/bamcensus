use super::acs_tiger;
use crate::model::acs_tiger_output_row::AcsTigerOutputRow;
use bamcensus_acs::model::{AcsApiQueryParams, AcsGeoidQuery, AcsType};
use bamcensus_core::model::identifier::{Geoid, GeoidType};
use clap::Parser;
use itertools::Itertools;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AcsTigerAppCli {
    /// geoid or comma-delimited list of geoids
    #[arg(short, long, alias = "geoid")]
    pub geoids: String,
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
    /// if provided, token for ACS API (to avoid public rate limits)
    #[arg(short, long)]
    pub acs_token: Option<String>,
    /// path and file to write result. if not provided, will use a concatenation of the CLI arguments
    #[arg(long)]
    pub output_file: Option<String>,
}

impl AcsTigerAppCli {
    pub async fn run(&self) {
        let acs_get_query = self.acs_query.split(',').map(String::from).collect_vec();
        let geoids = self
            .geoids
            .split(',')
            .map(|s| Geoid::try_from(s.trim()))
            .collect::<Result<Vec<Geoid>, _>>()
            .unwrap();
        let queries = geoids
            .into_iter()
            .map(|geoid| {
                let query: AcsGeoidQuery = AcsGeoidQuery::new(Some(geoid), self.output_resolution)?;
                let query_params = AcsApiQueryParams::new(
                    None,
                    self.year,
                    self.acs_type,
                    acs_get_query.clone(),
                    query,
                    self.acs_token.clone(),
                );
                Ok(query_params)
            })
            .collect::<Result<Vec<_>, String>>()
            .unwrap();

        let res_msg = match self.output_resolution {
            Some(res) => res.to_string(),
            None => String::new(),
        };
        let filename = match &self.output_file {
            None => format!("{}-{}-{}.csv", self.year, self.acs_type, res_msg),
            Some(f) => f.clone(),
        };
        let res = acs_tiger::run_batch(&queries).await.unwrap();
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
