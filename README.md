<div align="left">
  <a href="https://crates.io/crates/bamcensus">
    <img src="https://img.shields.io/crates/v/bamcensus" alt="Crates.io Latest Release"/>
  </a>
</div>

# BAMCensus - The Behavior and Advanced Mobility Census Dataset Aggregator
A high-performance toolkit written in Rust for downloading, aggregating, and spatially joining large-scale US Census datasets.

`bamcensus` addresses scaling limitations found in existing tools, such as R's [tidycensus](https://walker-data.com/tidycensus/), by providing concurrent batch downloads, hierarchical GEOID aggregation, and geometry joins between US Census tabular datasets (ACS and LEHD/LODES) and their corresponding geometries hosted on the Census TIGER/Line servers.

Its primary motivation stems from the need for a high-performance solution to combine spatial Census datasets with graph traversals within the context of mobility analysis tooling being developed at NLR's Behavior and Advanced Mobility (BAM) group.

## Key Features

- **Extensible Algebraic Data Types (ADTs)** for representing Census, ACS, LEHD/LODES, FIPS, and TIGER/Line taxonomies.
- **Automated TIGER/Line Geometry Joins** with geometries typed via `geo-types` and exported as WKT or `geopandas.GeoDataFrame`.
- **Local TIGER/Line Caching** (`~/.cache/bamcensus/tiger/` or `$BAMCENSUS_CACHE_DIR`) to avoid redundant shapefile downloads across runs.
- **Configurable LODES Year Fallback Policies** to handle state/year coverage gaps in LEHD datasets (see [LODES Fallback Policy](./docs/content/LODES_fallback_policy.md)).
- **Multiple Interfaces**:
  - Command-Line Interface (CLI) for batch downloading and CSV export
  - Rust library crates for programmatic workflows
  - Python bindings (`bamcensus-py`) returning `geopandas.GeoDataFrame` objects

## Workspace Crates

| Crate | Description |
| --- | --- |
| [`bamcensus`](./bamcensus/) | End-to-end ACS + TIGER and LODES + TIGER workflows and CLI binaries (`bamcensus`, `acs_tiger_app`, `lodes_tiger_app`) |
| [`bamcensus-core`](./bamcensus-core/) | Core FIPS/GEOID hierarchy models, state codes, and numeric aggregation traits |
| [`bamcensus-acs`](./bamcensus-acs/) | American Community Survey (ACS) 1-Year and 5-Year API query builders, client, and aggregators |
| [`bamcensus-lehd`](./bamcensus-lehd/) | LEHD Origin-Destination Employment Statistics (LODES) query builders, fallback policies, and block-to-parent aggregators |
| [`bamcensus-tiger`](./bamcensus-tiger/) | TIGER/Line shapefile resource resolution (2010–2020+ vintages), downloader, and shapefile parser |
| [`bamcensus-py`](./bamcensus-py/) | PyO3/Maturin Python bindings exposing `run_acs_tiger` and `run_wac_tiger` with `geopandas` integration |

## Geographic Resolution vs. Wildcard Semantics

Across the CLI, Rust API, and Python API, you specify:
1. **Input Region (`geoids`)**: One or more parent or target GEOIDs (e.g., `08` for Colorado, or `08031,08059` for Denver and Jefferson counties).
2. **Output Resolution (`--output-resolution` / `wildcard`)**: The target `GeoidType` granularity (`state`, `county`, `county_subdivision`, `place`, `census_tract`, `block_group`, `block`) for the returned rows and TIGER geometries:
   - **In ACS workflows**: Output resolution acts as a **server-side wildcard** (`for=<output_resolution>:*&in=...`) in the Census REST API query, returning estimates directly at the requested geography level.
   - **In LODES workflows**: Raw LODES files are always published by state at the **Census Block** level. When an output resolution (such as `census_tract` or `county`) is specified, `bamcensus` downloads the state LODES file, filters to blocks within the input `geoids`, **truncates block GEOIDs to the target parent `GeoidType`**, and aggregates job counts client-side before joining with TIGER/Line geometries at that resolution.

## Usage

- **CLI & Rust API**: See [`bamcensus/README.md`](./bamcensus/README.md)
- **Python API**: See [`bamcensus-py/README.md`](./bamcensus-py/README.md) and the notebooks in [`examples/`](./examples/)
- **LODES Fallback Policies**: See [`docs/content/LODES_fallback_policy.md`](./docs/content/LODES_fallback_policy.md)

## Roadmap

  - [x] open source release of beta version
  - [x] Python API (`bamcensus-py`)
  - [ ] readthedocs documentation
  - [ ] R API
  - [ ] macro-driven compilation of annual data schemas
  - [ ] support for OD + RAC LEHD datasets
  - [ ] polygon-to-GEOID spatial query filtering

## License

Copyright (c) 2025 Alliance for Sustainable Energy, LLC

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS “AS IS” AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
