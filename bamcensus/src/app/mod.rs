//! End-to-end download workflows for US Census datasets.
//!
//! An API for programmatically executing bamcensus from Rust code. see
//! the following functions for an entry point:
//!   - American Community Survey (ACS) with Tiger/LINES geometries: [`crate::app::acs_tiger::run`]
//!   - Longitudinal Employer-Household Dynamics (LEHD):
//!     - Origin-Destination Employment Statistics (LODES) [`crate::app::lodes_tiger::run`]

pub mod acs_tiger;
pub mod acs_tiger_args;
pub mod lodes_tiger;
pub mod lodes_tiger_args;
