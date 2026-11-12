use crate::py_args::parse_spatial_predicate;
use bamcensus_core::model::identifier::{GeoidType, HasGeoidString};
use bamcensus_tiger::model::TigerResourceBuilder;
use bamcensus_tiger::ops::geometry_io::parse_geometry_input;
use bamcensus_tiger::ops::spatial_resolver;
use pyo3::types::{PyDict, PyList};
use pyo3::{exceptions::PyException, prelude::*};
use reqwest::Client;
use wkt::ToWkt;

#[pyfunction]
#[pyo3(signature = (year, polygon, output_resolution, spatial_predicate="intersects", min_overlap=None, include_geometry=false))]
pub fn resolve_geoids_python<'a>(
    year: u64,
    polygon: &str,
    output_resolution: &str,
    spatial_predicate: &str,
    min_overlap: Option<f64>,
    include_geometry: bool,
    py: Python<'a>,
) -> PyResult<Bound<'a, PyList>> {
    let target_type: GeoidType = serde_json::from_str(output_resolution.trim())
        .or_else(|_| {
            serde_json::from_value(serde_json::Value::String(
                output_resolution.trim().to_string(),
            ))
        })
        .map_err(|e| {
            PyException::new_err(format!(
                "failure decoding 'output_resolution' argument from '{output_resolution}': {e}"
            ))
        })?;
    let predicate = parse_spatial_predicate(Some(spatial_predicate), min_overlap)
        .map_err(PyException::new_err)?;
    let geometry = parse_geometry_input(polygon).map_err(PyException::new_err)?;
    let builder = TigerResourceBuilder::new(year).map_err(PyException::new_err)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| {
            PyException::new_err(format!("failure creating async rust tokio runtime: {e}"))
        })?;

    let client = Client::new();
    let resolved = runtime
        .block_on(spatial_resolver::resolve_polygon(
            &client,
            &builder,
            &geometry,
            target_type,
            predicate,
        ))
        .map_err(|e| PyException::new_err(format!("failure resolving polygon to GEOIDs: {e}")))?;

    if include_geometry {
        let rows = resolved
            .matched_geometries
            .into_iter()
            .map(|(geoid, geom)| {
                let dict = PyDict::new_bound(py);
                dict.set_item("geoid", geoid.geoid_string())?;
                dict.set_item("geometry", geom.to_wkt().to_string())?;
                Ok(dict)
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(PyList::new_bound(py, rows))
    } else {
        let geoids: Vec<String> = resolved
            .matched_geometries
            .into_iter()
            .map(|(geoid, _)| geoid.geoid_string())
            .collect();
        Ok(PyList::new_bound(py, geoids))
    }
}
