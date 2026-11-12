use pyo3::prelude::*;
mod acs_tiger_python;
mod wac_tiger_python;

#[pymodule]
#[pyo3(name = "bamcensus_py")]
fn bamcensus_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(wac_tiger_python::run_wac_tiger_python, m)?)?;
    m.add_function(wrap_pyfunction!(acs_tiger_python::run_acs_tiger_python, m)?)?;
    Ok(())
}
