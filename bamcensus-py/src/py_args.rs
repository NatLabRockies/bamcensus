use bamcensus_core::model::identifier::{Geoid, GeoidType};
use bamcensus_core::model::spatial::{RegionFilter, SpatialPredicate};
use bamcensus_tiger::ops::geometry_io::parse_geometry_input;
use itertools::Itertools;
use pyo3::types::PyDict;
use pyo3::{exceptions::PyException, prelude::*};
use serde::de;
use std::str::FromStr;

pub fn get_opt_string(key: &str, map: Option<&Bound<'_, PyDict>>) -> PyResult<Option<String>> {
    let Some(map) = map else {
        return Ok(None);
    };
    let item_opt = map
        .get_item(key)
        .map_err(|e| PyException::new_err(format!("failure retrieving key {key}: {e}")))?;
    let Some(item) = item_opt else {
        return Ok(None);
    };
    if item.is_none() {
        return Ok(None);
    }
    let string: String = item
        .extract()
        .map_err(|e| PyException::new_err(format!("value at {key} is not string. error: {e}")))?;
    Ok(Some(string))
}

pub fn get_opt_f64(key: &str, map: Option<&Bound<'_, PyDict>>) -> PyResult<Option<f64>> {
    let Some(map) = map else {
        return Ok(None);
    };
    let item_opt = map
        .get_item(key)
        .map_err(|e| PyException::new_err(format!("failure retrieving key {key}: {e}")))?;
    let Some(item) = item_opt else {
        return Ok(None);
    };
    if item.is_none() {
        return Ok(None);
    }
    if let Ok(value) = item.extract::<f64>() {
        return Ok(Some(value));
    }
    let string: String = item
        .extract()
        .map_err(|e| PyException::new_err(format!("value at {key} is not float. error: {e}")))?;
    string
        .trim()
        .parse::<f64>()
        .map(Some)
        .map_err(|e| PyException::new_err(format!("invalid float at {key}: {e}")))
}

pub fn get_opt_string_deserializable<T>(
    key: &str,
    map: Option<&Bound<'_, PyDict>>,
) -> PyResult<Option<T>>
where
    T: de::DeserializeOwned,
{
    let Some(string) = get_opt_string(key, map)? else {
        return Ok(None);
    };
    let trimmed = string.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let t: T = serde_json::from_str(trimmed)
        .or_else(|_| serde_json::from_value(serde_json::Value::String(trimmed.to_string())))
        .map_err(|e| {
            PyException::new_err(format!(
                "failure decoding '{key}' argument from string '{string}': {e}"
            ))
        })?;
    Ok(Some(t))
}

pub fn get_raw_comma_separated(
    key: &str,
    map: Option<&Bound<'_, PyDict>>,
) -> PyResult<Option<Vec<String>>> {
    let Some(ss) = get_opt_string(key, map)? else {
        return Ok(None);
    };
    Ok(Some(
        ss.split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect_vec(),
    ))
}

pub fn get_comma_separated<T>(
    key: &str,
    map: Option<&Bound<'_, PyDict>>,
) -> PyResult<Option<Vec<T>>>
where
    T: de::DeserializeOwned,
{
    let Some(ss) = get_opt_string(key, map)? else {
        return Ok(None);
    };
    let result = ss
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|trimmed| {
            serde_json::from_str(trimmed)
                .or_else(|_| serde_json::from_value(serde_json::Value::String(trimmed.to_string())))
                .map_err(|e| format!("failure decoding comma-separated arguments in '{key}': {e}"))
        })
        .collect::<Result<Vec<T>, String>>();
    result.map(Some).map_err(|e| {
        PyException::new_err(format!(
            "failure decoding '{key}' argument as comma-separated list: {e}"
        ))
    })
}

pub fn get_output_resolution(map: Option<&Bound<'_, PyDict>>) -> PyResult<Option<GeoidType>> {
    if let Some(res) = get_opt_string_deserializable("wildcard", map)? {
        return Ok(Some(res));
    }
    get_opt_string_deserializable("output_resolution", map)
}

pub fn parse_spatial_predicate(
    predicate_str: Option<&str>,
    min_overlap: Option<f64>,
) -> Result<SpatialPredicate, String> {
    let raw = predicate_str.unwrap_or("intersects").trim();
    let is_bare_area_overlap = matches!(raw, "area_overlap" | "area-overlap");
    let is_prefixed_area_overlap =
        raw.starts_with("area_overlap:") || raw.starts_with("area-overlap:");

    if let Some(min_fraction) = min_overlap {
        if is_bare_area_overlap {
            let predicate = SpatialPredicate::AreaOverlap { min_fraction };
            predicate.validate()?;
            return Ok(predicate);
        } else if is_prefixed_area_overlap {
            return Err(
                "specify overlap threshold via either spatial_predicate='area_overlap:<fraction>' or min_overlap=<fraction>, not both"
                    .to_string(),
            );
        } else {
            return Err(format!(
                "min_overlap can only be used with spatial_predicate='area_overlap' (found '{raw}')"
            ));
        }
    }

    if is_bare_area_overlap {
        return Err(
            "area_overlap predicate requires a threshold via spatial_predicate='area_overlap:<fraction>' or min_overlap=<fraction>"
                .to_string(),
        );
    }

    SpatialPredicate::from_str(raw)
}

pub fn get_region_filter(map: Option<&Bound<'_, PyDict>>) -> PyResult<RegionFilter> {
    let geoids_opt = get_opt_string("geoids", map)?
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let polygon_opt = get_opt_string("polygon", map)?
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if geoids_opt.is_some() && polygon_opt.is_some() {
        return Err(PyException::new_err(
            "cannot specify both 'geoids' and 'polygon'",
        ));
    }

    if let Some(polygon_input) = polygon_opt {
        let predicate_str = get_opt_string("spatial_predicate", map)?;
        let min_overlap = get_opt_f64("min_overlap", map)?;
        let predicate = parse_spatial_predicate(predicate_str.as_deref(), min_overlap)
            .map_err(PyException::new_err)?;
        let geometry = parse_geometry_input(&polygon_input).map_err(PyException::new_err)?;
        return RegionFilter::new_geometry(geometry, predicate).map_err(PyException::new_err);
    }

    let geoids = match geoids_opt {
        None => vec![],
        Some(geoids_string) => geoids_string
            .split(',')
            .map(|s| Geoid::try_from(s.trim()))
            .collect::<Result<Vec<_>, String>>()
            .map_err(|e| PyException::new_err(format!("failure decoding geoids argument: {e}")))?,
    };
    Ok(RegionFilter::Geoids(geoids))
}
