//! bamcensus-tiger
//!
//! Tools for downloading shapefile datasets from the TIGER/Lines collection
//! served via FTP from <https://www2.census.gov/geo/tiger/>. A scalable and batch-based downloader
//! alternative to the PHP web interface found at <https://www.census.gov/cgi-bin/geo/shapefiles/index.php>.
//!
//! See <https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html> for
//! details on these datasets.
//!
//! # Usage
//!
//! TIGER/Line datasets can be downloaded using the [`crate::ops::tiger_api`] module `run` method.
//!
//! # Data model
//!
//! ## [TigerResourceBuilder]
//!
//! URIs to shapefiles and their contents vary based on the provided year. This type provides a method `create_resource` that
//! takes a single GEOID and returns a [TigerResource] to the shapefile matching that GEOID. the caller is expected
//! to know what are valid year / GEOID combinations but `create_resource` attempts to fail on all invalid combinations.
//!
//! ## [TigerResource]
//!
//! Once a resource object is created, it can be used to execute a download. The resource object provides knowledge about the
//! type of data stored in each row of the file, in order to map the result back to bamcensus types.
//!
//! [TigerResourceBuilder]: crate::model::TigerResourceBuilder
//! [TigerResource]: crate::model::TigerResource

pub mod model;
pub mod ops;
