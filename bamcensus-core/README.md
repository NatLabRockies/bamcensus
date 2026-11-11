# bamcensus-core

Core types, identifiers, and numeric aggregation operations shared across the [`bamcensus`](https://github.com/NREL/bamcensus) workspace.

## Overview

- **`model::identifier`**:
  - [`Geoid`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/enum.Geoid.html): Hierarchical Algebraic Data Type representing US Census geographic identifiers (`State`, `County`, `CountySubdivision`, `Place`, `CensusTract`, `BlockGroup`, `Block`).
  - [`GeoidType`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/enum.GeoidType.html): Enumeration of supported Census geographic hierarchy levels.
  - [`fips`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/fips/index.html): Strongly-typed newtypes for individual FIPS code components (`State`, `County`, `CensusTract`, etc.).
  - [`StateCode`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/enum.StateCode.html): US states and DC with conversions between FIPS codes, 2-letter postal abbreviations, and full names.
  - [`HasGeoidString`](https://docs.rs/bamcensus-core/latest/bamcensus_core/model/identifier/trait.HasGeoidString.html): Formats a `Geoid` or `fips` component as its canonical zero-padded FIPS string (e.g., `geoid.geoid_string()` returns `"08031"`, whereas `Display::fmt` / `to_string()` returns `"county=08031"`).
- **`model::spatial`**:
  - `RegionFilter` represents explicit GEOIDs or a polygonal region with a `SpatialPredicate`. Use `RegionFilter::new_geometry` to validate longitude/latitude coordinate ranges and normalize polygonal geometry collections; directly constructed or deserialized values should be validated before use.
  - `SpatialPredicate` supports `intersects`, `centroid_within`, `within`, and `area_overlap:<fraction>` (fraction in `(0, 1]`). Hyphens are also accepted in predicate names. Coordinate checks cannot determine the CRS or validate polygon topology.
- **`ops::agg`**:
  - [`NumericAggregation`](https://docs.rs/bamcensus-core/latest/bamcensus_core/ops/agg/enum.NumericAggregation.html): Aggregation functions (`Sum`, `Mean`, `Median`) for rolling up values when truncating child GEOIDs to parent geographies.
