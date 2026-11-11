# LODES Fallback Policy

## Overview

The LODES dataset has incomplete coverage across states and years. Some state/year combinations are missing entirely (e.g., Alaska data is unavailable for 2017-2021, Michigan is unavailable for 2022-2024).

To handle queries that span multiple states and years, bamcensus provides a configurable **fallback policy** that automatically attempts alternative years when a requested state/year is unavailable.

See [Data Availability](./../../bamcensus-lehd/2026-11-10-data-availability.md) for specific gaps in the LODES dataset.

## Fallback Policy Types

### FailAlways (Default)

```
--fallback fail_always
```

Fails immediately if the requested year is unavailable. No retry attempts are made.

**Use when:** You need data for a specific year only, or when you want to be aware of gaps in coverage.

### Try Previous Years

```
--fallback try_N_years_back
```

If the requested year fails, attempts N years earlier, in reverse chronological order.

**Example:** `try_2_years_back` with year 2021 will attempt: 2021 → 2020 → 2019

**Use when:** You need recent or current data, but can accept slightly older data if necessary.

### Try Next Years

```
--fallback try_N_years_forward
```

If the requested year fails, attempts N years later, in chronological order.

**Example:** `try_1_years_forward` with year 2021 will attempt: 2021 → 2022

**Use when:** You're requesting historical data and can accept more recent alternatives.

### Bidirectional

```
--fallback bidirectional_N_back_M_forward
```

If the requested year fails, attempts both earlier and later years in a bidirectional pattern.

**Example:** `bidirectional_2_back_1_forward` with year 2021 will attempt: 2021 → 2020 → 2019 → 2022

**Use when:** You want maximum flexibility in data availability.

## Command-Line Examples

### Rust CLI

```bash
# Try 2 years back if the requested year is unavailable
lodes_tiger_app wac \
  --year 2023 \
  --geoids 08 \
  --fallback try_2_years_back

# Try 1 year forward or 1 year back
lodes_tiger_app wac \
  --year 2023 \
  --geoids 08 \
  --fallback bidirectional_1_back_1_forward

# Fail if 2023 is unavailable (default)
lodes_tiger_app wac \
  --year 2023 \
  --geoids 08
```

### Python API

```python
from bamcensus import run_wac_tiger_python

# Try 2 years back if the requested year is unavailable
result = run_wac_tiger_python(
    year=2023,
    geoids="08,06",
    fallback_policy="try_2_years_back"
)

# Try both directions
result = run_wac_tiger_python(
    year=2023,
    geoids="08,06",
    fallback_policy="bidirectional_1_back_1_forward"
)

# Default behavior (fail on unavailable year)
result = run_wac_tiger_python(
    year=2023,
    geoids="08,06"
)
```

## Implementation Details

### How It Works

When a LODES download returns a **404 Not Found** HTTP response:

1. The fallback policy's candidate years are generated in priority order
2. Each candidate year is attempted in sequence
3. The first successful download is used
4. A warning is printed if a different year was used
5. Fatal errors (non-404 HTTP errors, parsing failures) are not retried

### Error Handling

- **404 Errors**: Trigger retry attempts
- **HTTP Errors (400, 500, etc.)**: Fail immediately (not retried)
- **Parse Errors**: Fail immediately (not retried)
- **All Candidates Exhausted**: Returns an empty result (no error)

### Concurrency

Multiple state/year combinations are downloaded in parallel. Each URL query gets its own retry logic applied independently, so retries do not block other parallel downloads.

### Logging

When an alternative year is used, a warning is printed:

```
LODES data for year 2023 not available; using 2022 instead
```

## Use Cases

### Multi-State Analysis Spanning Years

Query employment data for Colorado and California across 2023-2025, but automatically fall back to 2022 if data is unavailable:

```bash
lodes_tiger_app wac \
  --year 2023 \
  --geoids 08,06 \
  --output-resolution county \
  --fallback try_1_years_back
```

### Known Gaps Handling

Alaska data is missing for 2017-2021. Query jobs across all states for 2020, automatically using 2016 or 2022:

```bash
lodes_tiger_app wac \
  --year 2020 \
  --fallback bidirectional_3_back_2_forward
```

### Ensure Consistent Data Year

Query employment data and require it be from exactly 2020 (no fallback):

```bash
lodes_tiger_app wac \
  --year 2020 \
  --fallback fail_always
```

## Configuration

### Default Behavior

If `--fallback` is not specified, `fail_always` is used (no retry attempts).

### Valid Policy Formats

| Format | Example | Meaning |
|--------|---------|---------|
| `fail_always` | `fail_always` | No retries |
| `try_N_years_back` | `try_2_years_back` | Up to 2 earlier years |
| `try_N_years_forward` | `try_3_years_forward` | Up to 3 later years |
| `bidirectional_N_back_M_forward` | `bidirectional_1_back_2_forward` | 1 earlier or 2 later years |

### Programmatic Usage (Rust)

```rust
use bamcensus_lehd::model::FallbackPolicy;
use std::str::FromStr;

// Parse from string
let policy = FallbackPolicy::from_str("try_2_years_back")?;

// Create directly
let policy = FallbackPolicy::TryPreviousYears(2);

// Get candidate years
let candidates = policy.candidate_years(2023);
// Returns: [2023, 2022, 2021]
```

## Notes

- The fallback policy only affects LODES data fetching. TIGER geometry data uses a fixed year based on the LODES edition.
- Fallback policies are per-URL; each state's data is independently retried if necessary.
- Future years are only attempted if explicitly configured with forward retry policies.
