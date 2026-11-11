pub mod constants;
mod fallback_policy;
mod lodes_dataset;
mod lodes_edition;
mod lodes_job_type;
mod lodes_query;
mod od_job_segment;
mod od_part;
pub mod od_row;
pub mod rac_row;
mod wac_row;
mod wac_segment;
mod wac_value;
mod workplace_segment;

pub use fallback_policy::FallbackPolicy;
pub use lodes_dataset::LodesDataset;
pub use lodes_edition::LodesEdition;
pub use lodes_job_type::LodesJobType;
pub use lodes_query::LodesQuery;
pub use od_job_segment::OdJobSegment;
pub use od_part::OdPart;
pub use wac_row::WacRow;
pub use wac_segment::WacSegment;
pub use wac_value::WacValue;
pub use workplace_segment::WorkplaceSegment;

pub const LATEST_YEAR: u64 = 2021;
pub const BASE_URL: &str = "https://lehd.ces.census.gov/data/lodes";
pub const ALL_STATES: [&str; 52] = [
    "al", "ak", "az", "ar", "ca", "co", "ct", "de", "dc", "fl", "ga", "hi", "id", "il", "in", "ia",
    "ks", "ky", "la", "me", "md", "ma", "mi", "mn", "ms", "mo", "mt", "ne", "nv", "nh", "nj", "nm",
    "ny", "nc", "nd", "oh", "ok", "or", "pa", "ri", "sc", "sd", "tn", "tx", "ut", "vt", "va", "wa",
    "wv", "wi", "wy", "pr",
];

pub const CONTINENTAL_STATES: [&str; 49] = [
    "al", "az", "ar", "ca", "co", "ct", "de", "dc", "fl", "ga", "id", "il", "in", "ia", "ks", "ky",
    "la", "me", "md", "ma", "mi", "mn", "ms", "mo", "mt", "ne", "nv", "nh", "nj", "nm", "ny", "nc",
    "nd", "oh", "ok", "or", "pa", "ri", "sc", "sd", "tn", "tx", "ut", "vt", "va", "wa", "wv", "wi",
    "wy",
];
