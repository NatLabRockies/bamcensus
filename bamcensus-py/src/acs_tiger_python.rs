use crate::py_args::{
    get_opt_string, get_opt_string_deserializable, get_output_resolution, get_raw_comma_separated,
    get_region_filter,
};
use bamcensus::app::acs_tiger::{self, AcsRegionQueryParams};
use bamcensus_acs::model::AcsType;
use itertools::Itertools;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyNone;
use pyo3::{exceptions::PyException, prelude::*};
use wkt::ToWkt;

#[pyfunction]
#[pyo3(signature = (year, **kwds))]
pub fn run_acs_tiger_python<'a>(
    year: u64,
    kwds: Option<&Bound<'a, PyDict>>,
    py: Python<'a>,
) -> PyResult<pyo3::Bound<'a, PyList>> {
    let acs_type = get_opt_string_deserializable("acs_type", kwds)?.unwrap_or(AcsType::FiveYear);
    // default: populations total
    let acs_get_query = if let Some(q) = get_raw_comma_separated("acs_query", kwds)? {
        q
    } else if let Some(q) = get_raw_comma_separated("variables", kwds)? {
        q
    } else if let Some(q) = get_raw_comma_separated("wac_segments", kwds)? {
        q
    } else {
        vec![String::from("B01001_001E")]
    };

    let region = get_region_filter(kwds)?;
    let output_resolution = get_output_resolution(kwds)?;
    let acs_api_token = match get_opt_string("acs_api_token", kwds)? {
        Some(t) => Some(t),
        None => get_opt_string("acs_token", kwds)?,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| {
            PyException::new_err(format!("failure creating async rust tokio runtime: {e}"))
        })?;

    let params = AcsRegionQueryParams {
        base_url: None,
        year,
        acs_type,
        acs_get_query,
        region,
        output_resolution,
        acs_token: acs_api_token,
    };

    let result = runtime
        .block_on(acs_tiger::run_region(&params))
        .map_err(|e| PyException::new_err(format!("failure running ACS + TIGER workflow: {e}")))?;

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
            let dict: Bound<'_, PyDict> = PyDict::new_bound(py);
            let value_json = row.acs_value.value;

            dict.set_item("geoid", row.geoid.to_string())?;
            dict.set_item("name", row.acs_value.name)?;
            match value_json {
                serde_json::Value::Null => dict.set_item("value", PyNone::get_bound(py)),
                serde_json::Value::Bool(b) => dict.set_item("value", b.to_object(py)),
                serde_json::Value::Number(n) => match (n.as_f64(), n.as_i64()) {
                    (None, Some(i)) => dict.set_item("value", i.to_object(py)),
                    (Some(f), None) => dict.set_item("value", f.to_object(py)),
                    _ => Err(PyException::new_err(format!(
                        "cannot convert JSON Number to Python representation: {n}"
                    ))),
                },
                serde_json::Value::String(s) => dict.set_item("value", s.to_object(py)),
                _ => dict.set_item("value", value_json.to_string()),
            }?;
            dict.set_item("geometry", row.geometry.to_wkt().to_string())?;
            Ok(dict)
        })
        .collect::<PyResult<Vec<_>>>()?;
    let out_list = PyList::new_bound(py, vals);
    Ok(out_list)
}
