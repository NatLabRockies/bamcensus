# bamcensus-tiger

Tooling to support batch geospatial data collection from the US Census [TIGER/Line Shapefiles](https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html) archive, as part of the [`bamcensus`](https://github.com/NREL/bamcensus) workspace.

## Overview

- **[`TigerResourceBuilder`](https://docs.rs/bamcensus-tiger/latest/bamcensus_tiger/model/enum.TigerResourceBuilder.html)**: Resolves a list of [`Geoid`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/enum.Geoid.html) values and a vintage year (`2010` format, `2011–2019` format, or `2020+` format) into the minimal deduplicated set of [`TigerResource`](https://docs.rs/bamcensus-tiger/latest/bamcensus_tiger/model/struct.TigerResource.html) archive URLs needed to cover the requested geographies.
- **[`tiger_api::run`](https://docs.rs/bamcensus-tiger/latest/bamcensus_tiger/ops/tiger_api/fn.run.html)**: Downloads `.zip` shapefile archives with bounded concurrency, caches archives locally under `~/.cache/bamcensus/tiger/` (or `$BAMCENSUS_CACHE_DIR/tiger/`), extracts `.shp` and `.dbf` streams in memory, handles `GEOID` / `GEOID20` / `GEOID10` column variations, and returns matching `(Geoid, geo_types::Geometry<f64>)` pairs.
