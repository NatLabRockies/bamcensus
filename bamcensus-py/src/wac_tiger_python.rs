use bamcensus::app::lodes_tiger;
use bamcensus_core::model::identifier::Geoid;
use bamcensus_lehd::model::{
    FallbackPolicy, LodesDataset, LodesEdition, LodesJobType, WacSegment, WorkplaceSegment,
};
use itertools::Itertools;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::{exceptions::PyException, prelude::*};
use serde::de;
use std::str::FromStr;
use wkt::ToWkt;

/// kwds example: https://pyo3.rs/main/function/signature#using-pyo3signature--
#[pyfunction]
#[pyo3(signature = (year, **kwds))]
pub fn run_wac_tiger_python<'a>(
    year: u64,
    kwds: Option<&Bound<'a, PyDict>>,
    py: Python<'a>,
) -> PyResult<pyo3::Bound<'a, PyDict>> {
    let dataset_result: Result<LodesDataset, PyErr> = kwds
        .map(|m| {
            if m.contains("edition")? && m.contains("job_type")? && m.contains("segment")? {
                let edition: LodesEdition = get_string_deserializable("edition", m)?;
                let job_type: LodesJobType = get_string_deserializable("job_type", m)?;
                let segment: WorkplaceSegment = get_string_deserializable("segment", m)?;
                let dataset = LodesDataset::WAC {
                    edition,
                    job_type,
                    segment,
                    year,
                };
                Ok(dataset)
            } else {
                Ok(LodesDataset::default())
            }
        })
        .unwrap_or_else(|| Ok(LodesDataset::default()));
    let dataset = dataset_result?;

    let geoids_string: String = kwds.map_or(Ok(String::from("")), |m| get_string("geoids", m))?;
    let geoids = geoids_string
        .split(',')
        .map(Geoid::try_from)
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| PyException::new_err(format!("failure decoding geoids argument: {e}")))?;
    let wac_segments = kwds.map_or(Ok(vec![WacSegment::C000]), |m| {
        if m.contains("wac_segments")? {
            get_comma_separated("wac_segments", m)
        } else {
            Ok(vec![WacSegment::default()])
        }
    })?;
    let wildcard = kwds.map_or(Ok(None), |m| {
        if m.contains("wildcard")? {
            get_string_deserializable("wildcard", m).map(Some)
        } else {
            Ok(None)
        }
    })?;
    let fallback_policy = kwds.map_or(Ok(FallbackPolicy::FailAlways), |m| {
        if m.contains("fallback_policy")? {
            let policy_str: String = get_string("fallback_policy", m)?;
            FallbackPolicy::from_str(&policy_str)
                .map_err(|e| PyException::new_err(format!("invalid fallback_policy argument: {e}")))
        } else {
            Ok(FallbackPolicy::FailAlways)
        }
    })?;

    let future = lodes_tiger::run(&geoids, &wildcard, &wac_segments, &dataset, fallback_policy);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| {
            PyException::new_err(format!("failure creating async rust tokio runtime: {e}"))
        })?;
    let result = runtime.block_on(future).map_err(|e| {
        PyException::new_err(format!("failure running LODES WAC + TIGER workflow: {e}"))
    })?;

    if !result.tiger_errors.is_empty() {
        let msg = result.tiger_errors.iter().join(",");
        return Err(PyException::new_err(format!("tiger errors: {msg}")));
    }
    if !result.join_errors.is_empty() {
        let msg = result.join_errors.iter().join(",");
        return Err(PyException::new_err(format!("join errors: {msg}")));
    }

    let vals = result
        .join_dataset
        .into_iter()
        .map(|row| {
            let dict = PyDict::new_bound(py);
            dict.set_item("segment", row.value.segment.to_string())?;
            dict.set_item("value", row.value.value)?;
            dict.set_item("geometry", row.geometry.to_wkt().to_string())?;
            Ok((row.geoid.to_string(), dict.to_object(py)))
        })
        .collect::<PyResult<Vec<_>>>()?;
    let out_dict = vals.into_py_dict_bound(py);
    Ok(out_dict)
}

fn get_comma_separated<T>(key: &str, map: &Bound<'_, PyDict>) -> PyResult<Vec<T>>
where
    T: de::DeserializeOwned,
{
    let ss: String = get_string(key, map)?;
    let result = ss
        .split(',')
        .map(|s| {
            serde_json::from_str(s)
                .map_err(|e| format!("failure decoding comma-separated arguments in '{key}': {e}"))
        })
        .collect::<Result<Vec<T>, String>>();
    result.map_err(|e| {
        PyException::new_err(format!(
            "failure decoding '{key}' argument as comma-separated list: {e}"
        ))
    })
}

fn get_string(key: &str, map: &Bound<'_, PyDict>) -> PyResult<String> {
    let item_opt = map
        .get_item(key)
        .map_err(|e| PyException::new_err(format!("failure retreiving key {key}: {e}")))?;
    let item = match item_opt {
        None => Err(PyException::new_err(format!("key {key} not present"))),
        Some(item) => Ok(item),
    }?;
    let string: String = item
        .extract()
        .map_err(|e| PyException::new_err(format!("value at {key} is not string. error: {e}")))?;
    Ok(string)
}

fn get_string_deserializable<T>(key: &str, map: &Bound<'_, PyDict>) -> PyResult<T>
where
    T: de::DeserializeOwned,
{
    let item_opt = map
        .get_item(key)
        .map_err(|e| PyException::new_err(format!("failure retreiving key {key}: {e}")))?;
    let item = match item_opt {
        None => Err(PyException::new_err(format!("key {key} not present"))),
        Some(item) => Ok(item),
    }?;
    let string: String = item
        .extract()
        .map_err(|e| PyException::new_err(format!("value at {key} is not string. error: {e}")))?;
    let t: T = serde_json::from_str(string.as_str()).map_err(|e| {
        PyException::new_err(format!(
            "failure decoding '{key}' argument from string '{string}': {e}"
        ))
    })?;
    Ok(t)
}
