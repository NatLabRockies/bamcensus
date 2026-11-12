# bamcensus-lehd

Tooling to support batch geospatial data collection and aggregation from the US Census [Longitudinal Employer-Household Dynamics (LEHD)](https://lehd.ces.census.gov/) Origin-Destination Employment Statistics (LODES) datasets, as part of the [`bamcensus`](https://github.com/NLR/bamcensus) workspace.

## Overview

- **[`LodesDataset`](https://docs.rs/bamcensus-lehd/latest/bamcensus_lehd/model/enum.LodesDataset.html) & [`LodesQuery`](https://docs.rs/bamcensus-lehd/latest/bamcensus_lehd/model/struct.LodesQuery.html)**: Models LODES editions (`LODES6`, `LODES7`, `LODES8`), job types (`JT00`–`JT05`), workforce segments (`S000`, `SA01`–`SI03`), and WAC segments (`C000`, `CA01`–`CS02`), mapping each edition to its corresponding TIGER/Line geography vintage (`2010` for LODES6/7, `2021` for LODES8).
- **[`FallbackPolicy`](https://docs.rs/bamcensus-lehd/latest/bamcensus_lehd/model/enum.FallbackPolicy.html)**: Configurable retry policy (`FailAlways`, `TryPreviousYears`, `TryNextYears`, `BiDirectional`) when a requested state/year combination is unavailable. See [LODES Fallback Policy](../docs/content/LODES_fallback_policy.md) and [Data Availability](./2026-11-10-data-availability.md).
- **[`lodes_api`](https://docs.rs/bamcensus-lehd/latest/bamcensus_lehd/api/lodes_api/index.html)**: Downloads `.csv.gz` state LODES files concurrently and parses block-level employment counts.
- **[`lodes_agg`](https://docs.rs/bamcensus-lehd/latest/bamcensus_lehd/ops/lodes_agg/index.html)**: Truncates block-level GEOIDs to a target [`GeoidType`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/enum.GeoidType.html) (`BlockGroup`, `CensusTract`, `County`, `State`) and aggregates employment counts.
