from collections.abc import Iterable, Mapping
import os
from typing import Any, Optional, Union

import geopandas as gpd
import pandas as pd
import shapely
from shapely import wkt
from shapely.geometry import shape
from shapely.geometry.base import BaseGeometry

try:
    import bamcensus_py as api
except ImportError:  # pragma: no cover - allows unit testing pure-Python helpers before building extension
    api = None


def _normalize_polygon(polygon: Any) -> str:
    if isinstance(polygon, (str, os.PathLike)):
        return os.fspath(polygon)

    if isinstance(polygon, (gpd.GeoDataFrame, gpd.GeoSeries)):
        series = polygon.geometry if isinstance(polygon, gpd.GeoDataFrame) else polygon
        series = series.dropna()
        if series.empty:
            raise ValueError("polygon GeoDataFrame/GeoSeries must not be empty")
        if series.crs is not None:
            epsg = series.crs.to_epsg()
            if epsg not in (4326, 4269):
                series = series.to_crs("EPSG:4326")
        merged = shapely.unary_union(series.to_numpy())
        if merged.is_empty:
            raise ValueError("polygon geometry must not be empty")
        return merged.wkt

    if isinstance(polygon, BaseGeometry):
        if polygon.is_empty:
            raise ValueError("polygon geometry must not be empty")
        return polygon.wkt

    if isinstance(polygon, Mapping) or hasattr(polygon, "__geo_interface__"):
        geo = polygon if isinstance(polygon, Mapping) else polygon.__geo_interface__
        if isinstance(geo, Mapping) and geo.get("type") == "FeatureCollection":
            features = geo.get("features") or []
            geoms = [
                shape(feat["geometry"])
                for feat in features
                if isinstance(feat, Mapping) and feat.get("geometry") is not None
            ]
            if not geoms:
                raise ValueError("FeatureCollection must contain at least one geometry")
            merged = shapely.unary_union(geoms)
        elif isinstance(geo, Mapping) and geo.get("type") == "Feature":
            if geo.get("geometry") is None:
                raise ValueError("GeoJSON Feature has no geometry")
            merged = shape(geo["geometry"])
        else:
            merged = shape(geo)
        if merged.is_empty:
            raise ValueError("polygon geometry must not be empty")
        return merged.wkt

    raise TypeError(
        f"Unsupported polygon type {type(polygon).__name__}; expected WKT/GeoJSON/path string, "
        "Shapely geometry, GeoDataFrame/GeoSeries, or GeoJSON mapping"
    )


def _normalize_kwargs(kwds: dict[str, Any]) -> dict[str, Any]:
    out = {k: v for k, v in kwds.items() if v is not None}

    if "geoids" in out:
        geoids = out["geoids"]
        if isinstance(geoids, str):
            normalized_geoids = geoids.strip()
        elif isinstance(geoids, Iterable):
            normalized_geoids = ",".join(
                str(item).strip() for item in geoids if item is not None and str(item).strip()
            )
        else:
            normalized_geoids = str(geoids).strip()

        if normalized_geoids:
            out["geoids"] = normalized_geoids
        else:
            out.pop("geoids", None)

    if "polygon" in out:
        out["polygon"] = _normalize_polygon(out["polygon"])

    if "geoids" in out and "polygon" in out:
        raise ValueError("Cannot specify both 'geoids' and 'polygon'")

    for list_key in ("acs_query", "variables", "wac_segments"):
        if list_key in out and not isinstance(out[list_key], str):
            if isinstance(out[list_key], Iterable):
                out[list_key] = ",".join(str(item).strip() for item in out[list_key])

    return out


def _require_api():
    if api is None:
        raise RuntimeError("bamcensus_py native extension is not built or installed")
    return api


def run_wac_tiger(year: int, **kwds) -> gpd.GeoDataFrame:
    """Query LODES WAC data and join with TIGER/Line geometries for GEOIDs or a polygon."""
    normalized = _normalize_kwargs(kwds)
    data = _require_api().run_wac_tiger_python(year, **normalized)
    df = pd.DataFrame.from_records(data)
    if not df.empty:
        df["geometry"] = df.geometry.apply(wkt.loads)
    df = gpd.GeoDataFrame(df, crs="EPSG:4326")
    return df


def run_acs_tiger(year: int, **kwds) -> gpd.GeoDataFrame:
    """Query ACS data and join with TIGER/Line geometries for GEOIDs or a polygon."""
    normalized = _normalize_kwargs(kwds)
    data = _require_api().run_acs_tiger_python(year, **normalized)
    df = pd.DataFrame.from_records(data)
    if not df.empty:
        df["geometry"] = df.geometry.apply(wkt.loads)
    df = gpd.GeoDataFrame(df, crs="EPSG:4326")
    return df


def resolve_geoids(
    year: int,
    polygon: Any,
    output_resolution: str,
    spatial_predicate: str = "intersects",
    min_overlap: Optional[float] = None,
    include_geometry: bool = False,
) -> Union[list[str], gpd.GeoDataFrame]:
    """Resolve a polygon directly to matching TIGER/Line GEOIDs (or a GeoDataFrame when include_geometry=True)."""
    polygon_str = _normalize_polygon(polygon)
    result = _require_api().resolve_geoids_python(
        year,
        polygon_str,
        output_resolution,
        spatial_predicate,
        min_overlap,
        include_geometry,
    )
    if not include_geometry:
        return list(result)
    df = pd.DataFrame.from_records(result)
    if not df.empty:
        df["geometry"] = df.geometry.apply(wkt.loads)
    else:
        df = pd.DataFrame({"geoid": pd.Series(dtype="str"), "geometry": pd.Series(dtype="object")})
    return gpd.GeoDataFrame(df, geometry="geometry", crs="EPSG:4326")


__all__ = ("run_wac_tiger", "run_acs_tiger", "resolve_geoids")
