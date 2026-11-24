# bamcensus-core

Core types, identifiers, and numeric aggregation operations shared across the [`bamcensus`](https://github.com/NLR/bamcensus) workspace.

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

## HTTP retries

`ops::http::build_client(RetryConfig)` and `build_client_from(reqwest_client, RetryConfig)`
support three retry options:

- `RetryConfig::Disabled`: no retry middleware.
- `RetryConfig::Default`: exponential backoff with full jitter, up to three retries
  after the initial request (four attempts total).
- `RetryConfig::Custom { builder, max_retries }`: supply an
  `ExponentialBackoffBuilder` and a retry count. The builder configures interval
  bounds, jitter, and exponential base; it does not itself hold a retry count.

`ExponentialBackoff`, `ExponentialBackoffBuilder`, `Jitter`, and
`ClientWithMiddleware` are re-exported from `ops::http`.
The existing `build_retry_client()` and `build_retry_client_from(client, max_retries)`
helpers remain available.

```rust
use bamcensus_core::ops::http::{build_client, ExponentialBackoff, RetryConfig};
use std::time::Duration;

let client = build_client(RetryConfig::Custom {
    builder: ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(500), Duration::from_secs(30)),
    max_retries: 5,
});
```

Retries use `reqwest-retry`'s transient HTTP/network error classification.
Middleware retries request execution, not response-body reads, JSON parsing,
or archive extraction after a response has been returned.
