# bamcensus-py

Python bindings for [`bamcensus`](https://github.com/NLR/bamcensus), built with [PyO3](https://pyo3.rs/) and [Maturin](https://www.maturin.rs/), returning Census + TIGER/Line results directly as `geopandas.GeoDataFrame` objects.

## Installation

From the `bamcensus-py` directory (inside a Python virtual or Conda environment):

```sh
pip install maturin geopandas pandas shapely
maturin develop --release
```

## Usage

### ACS + TIGER/Line (`bamcensus.run_acs_tiger`)

```python
import bamcensus

# Query 2020 5-Year ACS population (B01001_001E) at the census tract level
# for Jefferson (08059) and Ouray (08091) counties in Colorado.
# Requires CENSUS_API_TOKEN in the environment or acs_token="..."
gdf = bamcensus.run_acs_tiger(
    year=2020,
    geoids="08059,08091",
    output_resolution="census_tract",  # or wildcard="census_tract"
    acs_query="NAME,B01001_001E",
    acs_type="five_year",
)
```

### LODES WAC + TIGER/Line (`bamcensus.run_wac_tiger`)

```python
import bamcensus
from shapely.geometry import box

# Query 2020 LODES Workplace Area Characteristics (WAC) total jobs (C000),
# aggregated from census blocks to census tracts for Jefferson and Ouray counties
gdf = bamcensus.run_wac_tiger(
    year=2020,
    geoids="08059,08091",
    output_resolution="census_tract",  # or wildcard="census_tract"
    wac_segments="C000",
    fallback_policy="try_2_years_back",
)

# Query by polygon (accepts Shapely geometry, GeoDataFrame/GeoSeries in any CRS,
# GeoJSON dict, or WKT/GeoJSON/Shapefile path)
study_area = box(-104.99, 39.74, -104.98, 39.75)
gdf_poly = bamcensus.run_wac_tiger(
    year=2021,
    polygon=study_area,
    spatial_predicate="intersects",  # or "centroid_within", "within", "area_overlap:0.5"
    output_resolution="census_tract",
    wac_segments="C000",
)

# Standalone polygon-to-GEOID lookup
tract_geoids = bamcensus.resolve_geoids(
    year=2020,
    polygon=study_area,
    output_resolution="census_tract",
    spatial_predicate="centroid_within",
)
```

See the Jupyter notebooks in [`examples/`](../examples/) for complete data retrieval and plotting examples.
