# Bamcensus

Orchestration library and command-line executables that combine the `bamcensus-*` crates to query US Census datasets (ACS and LEHD/LODES), aggregate values across the Census geographic hierarchy, and join them with TIGER/Line geometries.

Can be used:
  1. As a command-line application (CLI) — see installation and CLI usage below
  2. Programmatically from Rust code — see [`bamcensus::app::acs_tiger`](https://docs.rs/bamcensus/latest/bamcensus/app/acs_tiger/) and [`bamcensus::app::lodes_tiger`](https://docs.rs/bamcensus/latest/bamcensus/app/lodes_tiger/)

## Rust polygon queries

The Rust `acs_tiger::run_region` and `lodes_tiger::run_region` workflows accept
`RegionFilter::Geometry` as well as `RegionFilter::Geoids`. For example:

```rust,no_run
use bamcensus::app::{acs_tiger, lodes_tiger};
use bamcensus_acs::model::AcsType;
use bamcensus_core::model::identifier::GeoidType;
use bamcensus_core::model::spatial::{RegionFilter, SpatialPredicate};
use bamcensus_lehd::model::{FallbackPolicy, LodesDataset, WacSegment};
use geo::{polygon, Geometry};

async fn example() -> Result<(), String> {
let polygon = polygon![
    (x: -104.99, y: 39.74), (x: -104.98, y: 39.74),
    (x: -104.98, y: 39.75), (x: -104.99, y: 39.75),
    (x: -104.99, y: 39.74),
];
let region = RegionFilter::new_geometry(Geometry::Polygon(polygon), SpatialPredicate::Intersects)?;
let acs = acs_tiger::run_region(&acs_tiger::AcsRegionQueryParams {
    base_url: None,
    year: 2021,
    acs_type: AcsType::FiveYear,
    acs_get_query: vec!["B01001_001E".into()],
    region: region.clone(),
    output_resolution: Some(GeoidType::CensusTract),
    acs_token: std::env::var("CENSUS_API_TOKEN").ok(),
}).await?;
let lodes = lodes_tiger::run_region(
    &region,
    &Some(GeoidType::CensusTract),
    &[WacSegment::C000],
    &LodesDataset::default().with_year(2021),
    FallbackPolicy::FailAlways,
).await?;
Ok(())
}
```

Polygon queries select whole output geographies; geometries and data values are
not clipped or area-weighted. ACS requires an output resolution and does not
support blocks. LODES WAC defaults to blocks and supports state, county, tract,
block group, and block; aggregation includes all blocks in each selected
geography. It uses the TIGER vintage associated with the LODES edition. Areas
in territories bamcensus cannot query are omitted and reported in `tiger_errors`;
TIGER resolver failures instead fail the query. A polygon with no matches returns
an empty result without requesting ACS or LODES data.

Coordinates must already be longitude/latitude in EPSG:4269 or EPSG:4326;
inputs must be topologically valid, with antimeridian-crossing polygons split
at the antimeridian. Predicates, including area overlap, are planar in
longitude/latitude rather than equal-area or geodesic. See the
[TIGER README](../bamcensus-tiger/README.md) for parsing and predicate details.

## Installation

Build the command-line applications using Cargo (via [rustup](https://rustup.rs/)):

```sh
cargo build -r
```

## CLI Usage

You can run workflows either through the unified `bamcensus` binary (`bamcensus acs ...` / `bamcensus lehd lodes wac ...` / `bamcensus tiger ...`) or via the standalone `acs_tiger_app` and `lodes_tiger_app` binaries.

### Output Resolution Semantics (`--output-resolution`)

Valid `GeoidType` values for `--output-resolution`:
`state`, `county`, `county-subdivision`, `place`, `census-tract`, `block-group`, `block` (note: ACS does not support `block`).

- **In ACS (`acs_tiger_app`)**: `--output-resolution` specifies the Census API wildcard level (`for=<resolution>:*`) within the parent `--geoids` (or the target geography resolution when querying with `--polygon`).
- **In LODES (`lodes_tiger_app`)**: Raw LODES data is downloaded at the Census Block level for each relevant state, filtered to the input `--geoids` or `--polygon`, and aggregated by summing values up to `--output-resolution` before joining with TIGER/Line geometries.

---

### `acs_tiger_app` (or `bamcensus acs`)

Queries American Community Survey (ACS) data and joins it with TIGER/Line geometries for the specified GEOIDs or polygonal study region.

**Arguments:**
- `--geoids` (alias `--geoid`, mutually exclusive with `--polygon`): Comma-separated list of GEOIDs (e.g., `08031,08059`).
- `--polygon` (mutually exclusive with `--geoids`): Inline WKT/GeoJSON string or path to a `.wkt`, `.geojson`, `.json`, or `.shp` file. Requires `--output-resolution`.
- `--spatial-predicate` (optional): `intersects` (default), `centroid-within`, `within`, or `area-overlap[:fraction]`.
- `--min-overlap` (optional): Overlap threshold in `(0, 1]` when `--spatial-predicate=area-overlap`.
- `--output-resolution` (optional for `--geoids`, required for `--polygon`; alias `--aggregation`): Geospatial resolution for output (e.g., `census-tract`, `county`).
- `--year` (required): Year of ACS and TIGER/Line data.
- `--acs-query` (required): Comma-separated ACS variables to retrieve (e.g., `NAME,B01001_001E`).
- `--acs-type` (required): `one-year` or `five-year` estimates.
- `--acs-token` (required if `CENSUS_API_TOKEN` is not set; env: `CENSUS_API_TOKEN`): API token for the Census ACS API.
- `--output-file` (optional): Output CSV file path (default: `<year>-<acs_type>-<resolution>.csv`).

**Examples:**
```sh
./target/release/acs_tiger_app \
  --geoids=08031,08059 \
  --year=2020 \
  --acs-query=NAME,B01001_001E \
  --acs-type=five-year \
  --output-resolution=census-tract \
  --output-file=output.csv

./target/release/bamcensus acs \
  --polygon="POLYGON ((-104.99 39.74, -104.98 39.74, -104.98 39.75, -104.99 39.75, -104.99 39.74))" \
  --spatial-predicate=intersects \
  --year=2020 \
  --acs-query=NAME,B01001_001E \
  --acs-type=five-year \
  --output-resolution=census-tract \
  --output-file=polygon_acs.csv
```

---

### `lodes_tiger_app` (or `bamcensus lehd lodes`)

Queries LEHD Origin-Destination Employment Statistics (LODES) data, aggregates block-level counts to the requested resolution, and joins with TIGER/Line geometries.

**Subcommands:**
- `wac`: Workplace Area Characteristics (WAC) data.

**WAC Arguments:**
- `--geoids` (optional, mutually exclusive with `--polygon`): Comma-separated list of GEOIDs (defaults to all US states + DC if neither `--geoids` nor `--polygon` is supplied).
- `--polygon` (optional, mutually exclusive with `--geoids`): Inline WKT/GeoJSON string or path to a `.wkt`, `.geojson`, `.json`, or `.shp` file.
- `--spatial-predicate` (optional): `intersects` (default), `centroid-within`, `within`, or `area-overlap[:fraction]`.
- `--min-overlap` (optional): Overlap threshold in `(0, 1]` when `--spatial-predicate=area-overlap`.
- `--output-resolution` (optional): Target `GeoidType` aggregation level (e.g., `county`, `census-tract`; defaults to `block`).
- `--year` (required): Year of LODES data.
- `--wac-segments` (optional): Comma-separated WAC segments (default: `C000` for total jobs).
- `--edition` (optional): LODES edition (`lodes6`, `lodes7`, `lodes8`; default: `lodes8`).
- `--segment` (optional): Workforce segment (`s000`, `sa01`, etc.; default: `s000`).
- `--jobtype` (optional): Job type (`jt00` through `jt05`; default: `jt00`).
- `--fallback` (optional): Fallback policy when a state/year is unavailable (`fail_always`, `try_N_years_back`, `try_N_years_forward`, `bidirectional_N_back_M_forward`; default: `fail_always`). See [LODES Fallback Policy](../docs/content/LODES_fallback_policy.md).

**Example:**
```sh
./target/release/lodes_tiger_app wac \
  --geoids=08031 \
  --year=2020 \
  --output-resolution=census-tract \
  --wac-segments=C000 \
  --fallback=try_2_years_back
```

---

### `bamcensus tiger`

Resolves a polygon directly to matching TIGER/Line GEOIDs (and optional WKT geometries) without running an ACS or LODES query.

```sh
./target/release/bamcensus tiger \
  --polygon="POLYGON ((-104.99 39.74, -104.98 39.74, -104.98 39.75, -104.99 39.75, -104.99 39.74))" \
  --year=2020 \
  --output-resolution=census-tract \
  --spatial-predicate=centroid-within
```

## TIGER/Line Shapefile Caching

Downloaded TIGER/Line `.zip` archives are cached locally in `~/.cache/bamcensus/tiger/` (or `$BAMCENSUS_CACHE_DIR/tiger/` if `BAMCENSUS_CACHE_DIR` is set) so subsequent runs reuse previously downloaded geometry files.


