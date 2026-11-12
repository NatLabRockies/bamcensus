# bamcensus-acs

Tooling to support batch geospatial data collection and aggregation from the US Census [American Community Survey (ACS)](https://www.census.gov/programs-surveys/acs.html) REST API, as part of the [`bamcensus`](https://github.com/NLR/bamcensus) workspace.

## Overview

- **[`AcsGeoidQuery`](https://docs.rs/bamcensus-acs/latest/bamcensus_acs/model/enum.AcsGeoidQuery.html)**: Validates combinations of parent [`Geoid`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/enum.Geoid.html) and target wildcard [`GeoidType`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/enum.GeoidType.html) to construct Census API `for=...&in=...` clauses and response deserializers.
- **[`AcsApiQueryParams`](https://docs.rs/bamcensus-acs/latest/bamcensus_acs/model/struct.AcsApiQueryParams.html)**: Encapsulates dataset year, [`AcsType`](https://docs.rs/bamcensus-acs/latest/bamcensus_acs/model/enum.AcsType.html) (`OneYear` / `FiveYear`), variable list (`get=...`), geographic filter (`for_query`), and optional API key.
- **[`acs_api`](https://docs.rs/bamcensus-acs/latest/bamcensus_acs/api/acs_api/index.html)**: Executes single (`run`) or bounded-concurrency batch (`batch_run`) ACS API requests and deserializes JSON array responses into `(Geoid, Vec<AcsValue>)` tuples.
- **[`acs_agg`](https://docs.rs/bamcensus-acs/latest/bamcensus_acs/ops/acs_agg/index.html)**: Supports client-side hierarchical GEOID truncation and numeric aggregation over ACS values.
