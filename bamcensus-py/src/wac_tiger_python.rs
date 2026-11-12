use crate::py_args::{
    get_comma_separated, get_opt_string, get_opt_string_deserializable, get_output_resolution,
    get_region_filter,
};
use bamcensus::app::lodes_tiger;
use bamcensus_lehd::model::{FallbackPolicy, LodesDataset, WacSegment};
use itertools::Itertools;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::{exceptions::PyException, prelude::*};
use std::str::FromStr;
use wkt::ToWkt;

/// kwds example: https://pyo3.rs/main/function/signature#using-pyo3signature--
#[pyfunction]
#[pyo3(signature = (year, **kwds))]
pub fn run_wac_tiger_python<'a>(
    year: u64,
    kwds: Option<&Bound<'a, PyDict>>,
    py: Python<'a>,
) -> PyResult<pyo3::Bound<'a, PyList>> {
    let edition = get_opt_string_deserializable("edition", kwds)?.unwrap_or_default();
    let job_type = match get_opt_string_deserializable("job_type", kwds)? {
        Some(jt) => jt,
        None => get_opt_string_deserializable("jobtype", kwds)?.unwrap_or_default(),
    };
    let segment = get_opt_string_deserializable("segment", kwds)?.unwrap_or_default();
    let dataset = LodesDataset::WAC {
        edition,
        job_type,
        segment,
        year,
    };

    let region = get_region_filter(kwds)?;
    let wac_segments =
        get_comma_separated("wac_segments", kwds)?.unwrap_or_else(|| vec![WacSegment::default()]);
    let wildcard = get_output_resolution(kwds)?;
    let fallback_policy = match get_opt_string("fallback_policy", kwds)? {
        Some(policy_str) => FallbackPolicy::from_str(&policy_str)
            .map_err(|e| PyException::new_err(format!("invalid fallback_policy argument: {e}")))?,
        None => FallbackPolicy::FailAlways,
    };

    let future =
        lodes_tiger::run_region(&region, &wildcard, &wac_segments, &dataset, fallback_policy);
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
            dict.set_item("geoid", row.geoid.to_string())?;
            dict.set_item("segment", row.value.segment.to_string())?;
            dict.set_item("value", row.value.value)?;
            dict.set_item("geometry", row.geometry.to_wkt().to_string())?;
            Ok(dict)
        })
        .collect::<PyResult<Vec<_>>>()?;
    let out_list = PyList::new_bound(py, vals);
    Ok(out_list)
}
