# Bamcensus

Orchestration library and command-line executables that combine the `bamcensus-*` crates to query US Census datasets (ACS and LEHD/LODES), aggregate values across the Census geographic hierarchy, and join them with TIGER/Line geometries.

Can be used:
  1. As a command-line application (CLI) — see installation and CLI usage below
  2. Programmatically from Rust code — see [`bamcensus::app::acs_tiger`](https://docs.rs/bamcensus/latest/bamcensus/app/acs_tiger/) and [`bamcensus::app::lodes_tiger`](https://docs.rs/bamcensus/latest/bamcensus/app/lodes_tiger/)

## Installation

Build the command-line applications using Cargo (via [rustup](https://rustup.rs/)):

```sh
cargo build -r
```

## CLI Usage

You can run workflows either through the unified `bamcensus` binary (`bamcensus acs ...` / `bamcensus lehd lodes wac ...`) or via the standalone `acs_tiger_app` and `lodes_tiger_app` binaries.

### Output Resolution Semantics (`--output-resolution`)

Valid `GeoidType` values for `--output-resolution`:
`state`, `county`, `county-subdivision`, `place`, `census-tract`, `block-group`, `block` (note: ACS does not support `block`).

- **In ACS (`acs_tiger_app`)**: `--output-resolution` specifies the Census API wildcard level (`for=<resolution>:*`) within the parent `--geoids`. If omitted, queries the exact `--geoids` provided.
- **In LODES (`lodes_tiger_app`)**: Raw LODES data is downloaded at the Census Block level for each relevant state, filtered to the input `--geoids`, and aggregated by summing values up to `--output-resolution` before joining with TIGER/Line geometries.

---

### `acs_tiger_app` (or `bamcensus acs`)

Queries American Community Survey (ACS) data and joins it with TIGER/Line geometries for the specified GEOIDs.

**Arguments:**
- `--geoids` (required, alias `--geoid`): Comma-separated list of GEOIDs (e.g., `08031,08059`).
- `--output-resolution` (optional, alias `--aggregation`): Geospatial resolution for output (e.g., `census-tract`, `county`). Uses the resolution of each input GEOID if omitted.
- `--year` (required): Year of ACS and TIGER/Line data.
- `--acs-query` (required): Comma-separated ACS variables to retrieve (e.g., `NAME,B01001_001E`).
- `--acs-type` (required): `one-year` or `five-year` estimates.
- `--acs-token` (optional): API token for the Census ACS API.
- `--output-file` (optional): Output CSV file path (default: `<year>-<acs_type>-<resolution>.csv`).

**Example:**
```sh
./target/release/acs_tiger_app \
  --geoids=08031,08059 \
  --year=2020 \
  --acs-query=NAME,B01001_001E \
  --acs-type=five-year \
  --output-resolution=census-tract \
  --output-file=output.csv
```

---

### `lodes_tiger_app` (or `bamcensus lehd lodes`)

Queries LEHD Origin-Destination Employment Statistics (LODES) data, aggregates block-level counts to the requested resolution, and joins with TIGER/Line geometries.

**Subcommands:**
- `wac`: Workplace Area Characteristics (WAC) data.

**WAC Arguments:**
- `--geoids` (optional): Comma-separated list of GEOIDs (defaults to all US states + DC).
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

## TIGER/Line Shapefile Caching

Downloaded TIGER/Line `.zip` archives are cached locally in `~/.cache/bamcensus/tiger/` (or `$BAMCENSUS_CACHE_DIR/tiger/` if `BAMCENSUS_CACHE_DIR` is set) so subsequent runs reuse previously downloaded geometry files.


